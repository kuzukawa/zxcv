use std::fs::OpenOptions;
use std::io::Write;
use std::sync::OnceLock;

static ENABLED: OnceLock<bool> = OnceLock::new();

pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| std::env::var("ZXCV_DEBUG").is_ok())
}

pub fn log(msg: impl AsRef<str>) {
    if !enabled() {
        return;
    }
    let path = std::env::var("ZXCV_DEBUG_LOG").unwrap_or_else(|_| "/tmp/zxcv-debug.log".into());
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "[{}] {}", std::process::id(), msg.as_ref());
    }
}
