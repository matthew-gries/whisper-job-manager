use std::path::{PathBuf, Path};

use serde::Deserialize;


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub video_storage_path: String,
    pub host: String,
    pub port: u16
}

pub fn read_config<P: AsRef<Path>>(config_path: P) -> Config {
    let mut path = PathBuf::new();
    path.push(config_path);
    let data = std::fs::read_to_string(path.as_path()).unwrap();
    let config: Config = serde_json::from_str(&data).unwrap();

    let video_storage_path = std::fs::canonicalize(&config.video_storage_path);

    if let Ok(v) = video_storage_path {
        if !v.is_dir() {
            panic!("{} is not a directory", config.video_storage_path);
        }
    } else {
        panic!("Cannot find canonical path for {}", config.video_storage_path);
    }

    config
}