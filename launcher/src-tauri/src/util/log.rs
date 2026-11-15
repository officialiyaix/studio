//! Minimal file logger with secret redaction. Access tokens, refresh tokens and authorization codes are registered
//! as secrets and replaced before any line is written, so they never appear in launcher or game launch logs.

use std::io::Write;
use std::sync::{Mutex, OnceLock};

static SECRETS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static FILE: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();

fn secrets() -> &'static Mutex<Vec<String>> {
    SECRETS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Register a value that must never be logged.
pub fn register_secret(value: &str) {
    if value.len() >= 8 {
        let mut s = secrets().lock().unwrap();
        if !s.iter().any(|x| x == value) {
            s.push(value.to_string());
        }
    }
}

pub fn redact(line: &str) -> String {
    let mut out = line.to_string();
    for s in secrets().lock().unwrap().iter() {
        if out.contains(s.as_str()) {
            out = out.replace(s.as_str(), "[redacted]");
        }
    }
    out
}

pub fn init() {
    let path = crate::paths::logs_dir().join("launcher.log");
    let _ = std::fs::create_dir_all(crate::paths::logs_dir());
    // Keep one previous log for troubleshooting.
    if path.exists() {
        let _ = std::fs::rename(&path, crate::paths::logs_dir().join("launcher.previous.log"));
    }
    let file = std::fs::OpenOptions::new().create(true).append(true).open(path).ok();
    let _ = FILE.set(Mutex::new(file));
}

pub fn write(level: &str, message: &str) {
    let line = format!("{} [{}] {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), level, redact(message));
    if cfg!(debug_assertions) {
        eprint!("{line}");
    }
    if let Some(m) = FILE.get() {
        if let Some(f) = m.lock().unwrap().as_mut() {
            let _ = f.write_all(line.as_bytes());
        }
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => { $crate::util::log::write("INFO", &format!($($arg)*)) };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => { $crate::util::log::write("WARN", &format!($($arg)*)) };
}
