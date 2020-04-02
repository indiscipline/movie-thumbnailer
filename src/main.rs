extern crate clap;
extern crate rayon;

mod config;

use rayon::prelude::*;
use std::env;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::Command;

struct Thumbnail {
    w: usize,
    h: usize,
    w_margin: usize,
    h_margin: usize,
    columns: usize,
}

fn main() {
    let cfg = config::get();

    let work_dir = env::current_dir().unwrap();
    let frames_dir = work_dir.join("frames");

    if cfg.is_present("INPUT") {
        let input_path = PathBuf::from(cfg.value_of("INPUT").unwrap().to_string());
        println!("Extracting scene change frames. This may take a while...");
        extract_scenes(&input_path, &frames_dir);
        println!("Please, manually remove falsely identified or unwanted scene change frames from the \"frames\" directory.");
        println!("Press any key to continue when done...");
        io::stdin().read_exact(&mut [0u8]).unwrap();
    } else {
        println!("No input file given, looking for saved frames...");
    }

    let preprocessed_dir = work_dir.join("frames_scaled");
    if preprocessed_dir.is_dir() && cfg.is_present("skip") {
        println!("Found directory with preprocessed files. Skipping.");
    } else {
        println!("Preprocessing extracted frames. This may take a while...");
        preprocess_frames(&work_dir, &preprocessed_dir);
    }

    let files = preprocessed_dir
        .read_dir()
        .expect("Error reading input frames!");
    let frames_q = files
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().unwrap_or_default().to_str() == Some("png"))
        .count();

    let (frame_w, frame_h) = get_source_resolution(if frames_dir.is_dir() {
        &frames_dir
    } else {
        &preprocessed_dir
    });

    let resolutions = parse_resolution(cfg.value_of("resolution").unwrap());
    resolutions.par_iter().for_each(|(display_w, display_h)| {
        let thumbnail = calc_thumbnail_size(*display_w, *display_h, frames_q, frame_w, frame_h);
        montage(
            &preprocessed_dir,
            &work_dir,
            &thumbnail,
            *display_w,
            *display_h,
        );
    });
}

fn calc_thumbnail_size(
    display_w: usize,
    display_h: usize,
    frames: usize,
    frame_w: usize,
    frame_h: usize,
) -> Thumbnail {
    // todo: replace hard-coded values
    let factor_w = 0.94;
    let factor_h = 0.82;
    let h_margin = (frame_w - frame_h) as f32 / (5 * frame_h) as f32; // vertical margin depends on W to H frame ratio
    let w_margin = 0.04;

    let area_w = display_w as f32 * factor_w;
    let area_h = display_h as f32 * factor_h;
    let area = area_w * area_h;
    let ratio = frame_w as f32 / frame_h as f32;

    let max_frame_area = area as f32 / frames as f32;
    let x = (max_frame_area
        / (ratio * (1. + 2. * w_margin + 2. * h_margin + 5. * w_margin * h_margin)))
        .sqrt();

    Thumbnail {
        w: (x.round() * ratio).round() as usize, //multiplying rounded x to avoid ratio distortion
        h: x.round() as usize,
        w_margin: (x * ratio * w_margin).round() as usize,
        h_margin: (x * h_margin).round() as usize,
        columns: (area_w / (x * ratio + x * ratio * w_margin * 2.)).round() as usize,
    }
}

fn parse_resolution(s: &str) -> Vec<(usize, usize)> {
    s.trim().split(',')
        .map(|r| {
            let res = r
                .splitn(2, 'x')
                .map(|i| i.parse::<usize>().unwrap())
                .collect::<Vec<usize>>();
            (res[0], res[1])
        })
        .collect()
}

fn get_source_resolution(source: &PathBuf) -> (usize, usize) {
    let mut command = Command::new("magick");
    command
        .current_dir(source)
        .arg("identify")
        .args(&["-ping", "-format", "%[w]x%[h]"])
        .arg(
            source
                .read_dir()
                .expect("Failed to read source frame resolution!")
                .next()
                .unwrap()
                .unwrap()
                .path()
                .to_str()
                .unwrap(),
        );
    let output = command.output().expect("failed to execute process");
    let res = parse_resolution(&String::from_utf8_lossy(&output.stdout));
    res[0]
}

// Line up thumbnails for given geometry
fn montage(
    source: &PathBuf,
    target: &PathBuf,
    thumb: &Thumbnail,
    display_w: usize,
    display_h: usize,
) {
    let mut command = Command::new("magick");
    command
        .current_dir(source)
        .arg("montage")
        .arg("*.png")
        .args(&[
            "-background",
            "#00000000",
            "-filter",
            "Lanczos2Sharp",
            "-unsharp",
            "0x3+1+0.01",
        ])
        .args(&[
            "-geometry",
            &format!(
                "{}x{}+{}+{}",
                thumb.w, thumb.h, thumb.w_margin, thumb.h_margin
            ),
        ])
        .args(&["-tile", &thumb.columns.to_string()])
        .arg(target.join(format!("montage-{}x{}.png", display_w, display_h)));
    //println!("{:?}", command);

    let output = command.output().expect("Failed to execute process");
    if !&output.stderr.is_empty() {
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
}

// Preprocess and resize frames for possible montages for different resolutions
fn preprocess_frames(source: &PathBuf, target: &PathBuf) {
    if !target.is_dir() {
        fs::create_dir(target).unwrap();
    }

    let entries = source
        .join("frames")
        .read_dir()
        .expect("Error reading input frames!");
    let process_list = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().unwrap_or_default().to_str() == Some("png"))
        .collect::<Vec<PathBuf>>();
    process_list.par_iter().for_each(|path| {
        let mut command = Command::new("magick");
        command
            .current_dir(target)
            .arg(path)
            .args(&[
                "-gamma",
                ".45455",
                "-despeckle",
                "-statistic",
                "Nonpeak",
                "1.75x1.75",
                "-wavelet-denoise",
                "18x0.06",
                "-adaptive-sharpen",
                "3",
                "+sigmoidal-contrast",
                "4,48%",
                "-filter",
                "Lanczos2Sharp",
                "-resize",
                "50%",
                "-unsharp",
                "0x2+1+0.04",
                "-linear-stretch",
                "0.3",
                "-modulate",
                "100x110",
                "-gamma",
                "2.2",
            ])
            .arg(path.file_name().unwrap());

        let output = command.output().expect("Failed to execute process");
        if !&output.stderr.is_empty() {
            println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
    });
}

// Create "frames" dir and extract the frames from scene changes
fn extract_scenes(source: &PathBuf, target: &PathBuf) {
    if !target.is_dir() {
        fs::create_dir(target).unwrap();
    }

    let mut command = Command::new("ffmpeg");
    command
        .current_dir(target)
        .arg("-i")
        .arg(source.to_str().expect("Error converting Path to unicode!"))
        .args(&[
            "-vf",
            "select='gt(scene\\,0.33)'",
            "-vsync",
            "vfr",
            "%06d.png",
        ]);

    let output = command.output().expect("failed to execute process");

    if !&output.stderr.is_empty() {
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
}
