use std::ffi::OsString;

use clap::Parser;

/// CLI program to run jobs with the Whisper job manager
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path within the storage folder on the server to run Whisper on, must be relative
    pub filepath: String,

    /// Endpoint to call
    #[arg(short, long, default_value_t = String::from("http://127.0.0.1:8080"))]
    pub endpoint: String,

    /// The directory to put the subtitle file in
    #[arg(short, long, default_value_t = String::from("output"))]
    pub output_dir: String,

    /// The name of the subtitle file to save
    #[arg(short, long)]
    pub name: Option<OsString>,

    /// The amount of time to wait before timing out, in milliseconds, defaults to 30 min
    #[arg(short, long, default_value_t = 1000 * 60 * 30)]
    pub timeout: u64,

    /// The amount of time to wait before timing out, in milliseconds, defaults to 1 min
    #[arg(short, long, default_value_t = 1000 * 60)]
    pub poll_interval: u64,
}
