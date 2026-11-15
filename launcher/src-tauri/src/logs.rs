//! A Save's logs for the launcher's log viewer: the game's own logs (`logs/latest.log`, `debug.log`, archived
//! `*.log.gz`), Stabled's capture of the game's console output (`logs/stabled-launch.log`), crash reports and
//! JVM crash files (`hs_err_pid*.log`). Files are only ever read, and only files from the listing can be opened.

use crate::bail;
use crate::error::AppResult;
use crate::saves::SaveMeta;
use serde::Serialize;
use std::io::Read;
use std::path::PathBuf;

/// Larger logs are cut to their last part; crashes are at the end.
const MAX_BYTES: u64 = 6 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFile {
    /// Path relative to the game directory, e.g. "logs/latest.log" or "crash-reports/crash-….txt".
    pub name: String,
    /// "latest" | "launch" | "debug" | "archive" | "crash" | "jvm-crash"
    pub kind: String,
    pub size: u64,
    pub modified: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogContent {
    pub name: String,
    pub text: String,
    pub truncated: bool,
    pub size: u64,
    pub modified: Option<String>,
}

fn modified(p: &std::path::Path) -> Option<String> {
    let t = std::fs::metadata(p).ok()?.modified().ok()?;
    Some(chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
}

pub fn list(meta: &SaveMeta) -> Vec<LogFile> {
    let game = meta.game_dir();
    let mut out: Vec<(LogFile, std::time::SystemTime)> = Vec::new();
    let mut push = |rel: String, kind: &str, path: PathBuf| {
        if let Ok(m) = std::fs::metadata(&path) {
            if m.is_file() {
                out.push((LogFile { name: rel, kind: kind.into(), size: m.len(), modified: modified(&path) }, m.modified().unwrap_or(std::time::UNIX_EPOCH)));
            }
        }
    };
    for e in std::fs::read_dir(game.join("logs")).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let kind = match name.as_str() {
            "latest.log" => "latest",
            "stabled-launch.log" => "launch",
            "debug.log" => "debug",
            n if n.ends_with(".log.gz") || n.ends_with(".log") => "archive",
            _ => continue,
        };
        push(format!("logs/{name}"), kind, e.path());
    }
    for e in std::fs::read_dir(game.join("crash-reports")).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.ends_with(".txt") {
            push(format!("crash-reports/{name}"), "crash", e.path());
        }
    }
    for e in std::fs::read_dir(&game).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with("hs_err_pid") && name.ends_with(".log") {
            push(name, "jvm-crash", e.path());
        }
    }
    // Current logs first, then everything else newest first.
    let rank = |k: &str| match k {
        "latest" => 0,
        "launch" => 1,
        _ => 2,
    };
    out.sort_by(|(a, at), (b, bt)| rank(&a.kind).cmp(&rank(&b.kind)).then(bt.cmp(at)));
    out.into_iter().map(|(f, _)| f).collect()
}

/// Reads one file from the listing. `.gz` archives are decompressed.
pub fn read(meta: &SaveMeta, name: &str, redact: Option<&Redactor>) -> AppResult<LogContent> {
    if !list(meta).iter().any(|f| f.name == name) {
        bail!("LOGS", "That log file isn't available.");
    }
    let path = meta.game_dir().join(name);
    let size = std::fs::metadata(&path)?.len();
    let (bytes, truncated) = if name.ends_with(".gz") {
        let mut d = flate2::read::GzDecoder::new(std::fs::File::open(&path)?);
        let mut buf = Vec::new();
        d.by_ref().take(MAX_BYTES * 4).read_to_end(&mut buf)?;
        let cut = buf.len() as u64 > MAX_BYTES;
        if cut {
            buf.drain(..buf.len() - MAX_BYTES as usize);
        }
        (buf, cut)
    } else {
        tail(&path, MAX_BYTES)?
    };
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if truncated {
        // Start at a whole line.
        if let Some(i) = text.find('\n') {
            text.drain(..=i);
        }
    }
    if let Some(r) = redact {
        text = r.apply(&text);
    }
    Ok(LogContent { name: name.into(), text, truncated, size, modified: modified(&path) })
}

/// The last `max` bytes of a file.
pub fn tail(path: &std::path::Path, max: u64) -> AppResult<(Vec<u8>, bool)> {
    use std::io::{Seek, SeekFrom};
    let mut f = std::fs::File::open(path)?;
    let len = f.metadata()?.len();
    let cut = len > max;
    if cut {
        f.seek(SeekFrom::Start(len - max))?;
    }
    let mut buf = Vec::with_capacity(len.min(max) as usize);
    f.take(max).read_to_end(&mut buf)?;
    Ok((buf, cut))
}

/// Streamer-mode redaction for logs: the player's name, the user folder and server addresses.
pub struct Redactor {
    name: Option<regex::Regex>,
    home: Option<String>,
    user_dir: regex::Regex,
    connect: regex::Regex,
    ipv4: regex::Regex,
}

impl Redactor {
    pub fn new(player_name: Option<&str>) -> Redactor {
        Redactor {
            name: player_name.filter(|n| n.len() >= 2).and_then(|n| regex::Regex::new(&format!(r"\b{}\b", regex::escape(n))).ok()),
            home: dirs::home_dir().map(|h| h.to_string_lossy().into_owned()).filter(|h| h.len() > 3),
            user_dir: regex::Regex::new(r"(?i)([\\/](?:Users|home)[\\/])[^\\/\s:]+").unwrap(),
            connect: regex::Regex::new(r"(?i)(Connecting to )[^\s,]+(, ?\d+)?").unwrap(),
            ipv4: regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap(),
        }
    }

    pub fn apply(&self, text: &str) -> String {
        let mut t = text.to_string();
        if let Some(h) = &self.home {
            t = t.replace(h.as_str(), "~");
        }
        t = self.user_dir.replace_all(&t, "${1}•••").into_owned();
        if let Some(n) = &self.name {
            t = n.replace_all(&t, "Player").into_owned();
        }
        t = self.connect.replace_all(&t, "${1}[server hidden]").into_owned();
        self.ipv4.replace_all(&t, "[address hidden]").into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::Redactor;

    #[test]
    fn streamer_redaction_hides_name_folders_and_servers() {
        let r = Redactor::new(Some("Alex_99"));
        let out = r.apply("[Render thread/INFO]: Setting user: Alex_99\nC:\\Users\\alex\\AppData\\Roaming\\Stabled\nConnecting to play.example.net, 25565\nfrom 192.168.1.20");
        assert!(!out.contains("Alex_99"));
        assert!(out.contains("Setting user: Player"));
        assert!(!out.contains("\\alex\\"));
        assert!(!out.contains("play.example.net"));
        assert!(!out.contains("192.168.1.20"));
    }
}
