use std::path::PathBuf;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref TMP_DIR: PathBuf = {
        let mut path = PathBuf::new();
        path.push("./tmp");
        path
    };
}
