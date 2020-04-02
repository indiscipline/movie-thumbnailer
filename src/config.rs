extern crate clap;

use clap::{App, Arg, crate_version, crate_authors};
use std::fs;
use std::process;

static LICENSE: &str =
    "Movie-wallpaper licensed under GNU General Public License version 3 or later;
Full License available at <http://gnu.org/licenses/gpl.html>.
This is free software: you are free to change and redistribute it.
There is NO WARRANTY, to the extent permitted by law.";

pub fn get<'a>() -> clap::ArgMatches<'a> {
    let cfg = App::new("movie-wallpaper")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Create a wallpaper composed of tiled frames of scene changes from supplied video.\nThis program depends on imagemagick ('magick') and ffmpeg executables!")
        .arg(
            Arg::with_name("INPUT")
                .help("Path to the input file to process.")
                .validator(validate_input_files)
                .required(false),
        )
        .arg(
            Arg::with_name("license")
                .short("l")
                .long("license")
                .help("Display the license information."),
        )
        .arg(
            Arg::with_name("skip")
                .short("s")
                .long("skip")
                .takes_value(false)
                .help("Skip preprocessing if directory exists."),
        )
        .arg(
            Arg::with_name("resolution")
                .short("r")
                .long("resolution")
                .takes_value(true)
                .help(
                    "Comma-separated list of target display resolution, in form [width]x[height].",
                )
                .required(true),
        )
        .get_matches();

    if cfg.is_present("license") {
        println!("{}", LICENSE);
        process::exit(0);
    }

    cfg
}

fn validate_input_files(a: String) -> Result<(), String> {
    match fs::metadata(a) {
        Ok(_) => Ok(()),
        Err(error) => panic!("There was a problem reading the file: {:?}", error),
    }
}
