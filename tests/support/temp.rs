use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn temp_dir(prefix: &str) -> PathBuf {
    let suffix = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => value.as_nanos(),
        Err(err) => panic!("system clock should be after unix epoch: {err}"),
    };
    let path = std::env::temp_dir().join(format!("{prefix}-{suffix}"));
    if let Err(err) = std::fs::create_dir_all(&path) {
        panic!("temp dir should be created: {err}");
    }
    path
}
