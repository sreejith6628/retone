//! Tiny append-only error/diagnostic log (never contains draft or result text).
use std::io::Write;
use std::sync::OnceLock;

static PATH: OnceLock<std::path::PathBuf> = OnceLock::new();

pub fn init(dir: std::path::PathBuf) {
    let _ = std::fs::create_dir_all(&dir);
    let _ = PATH.set(dir.join("retone.log"));
}

pub fn line(msg: impl AsRef<str>) {
    let Some(p) = PATH.get() else { return };
    // Keep the file small: start over past 256 KB.
    if std::fs::metadata(p).map(|m| m.len() > 256 * 1024).unwrap_or(false) { let _ = std::fs::remove_file(p); }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "{secs} {}", msg.as_ref());
    }
}
