//! HTTP client and verified downloads.
//!
//! Every file the launcher installs is checked against the hash published by its source (Mojang SHA-1, Fabric
//! SHA-1, Modrinth SHA-512) before it is moved into place, and files already present with the right hash are not
//! downloaded again.

use crate::bail;
use crate::error::{AppError, AppResult};
use futures_util::StreamExt;
use sha1::Digest;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(concat!("Stabled/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .build()
            .expect("HTTP client")
    })
}

#[derive(Clone, Debug)]
pub enum Hash {
    Sha1(String),
    Sha512(String),
    None,
}

#[derive(Clone, Debug)]
pub struct Download {
    pub url: String,
    pub path: PathBuf,
    pub hash: Hash,
    pub size: Option<u64>,
}

pub fn file_sha1(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(hex::encode(sha1::Sha1::digest(&bytes)))
}

pub fn file_sha512(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(hex::encode(sha2::Sha512::digest(&bytes)))
}

pub fn bytes_sha512(bytes: &[u8]) -> String {
    hex::encode(sha2::Sha512::digest(bytes))
}

pub fn is_valid(path: &Path, hash: &Hash, size: Option<u64>) -> bool {
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if let Some(s) = size {
        if meta.len() != s {
            return false;
        }
    }
    match hash {
        Hash::Sha1(h) => file_sha1(path).map(|x| x.eq_ignore_ascii_case(h)).unwrap_or(false),
        Hash::Sha512(h) => file_sha512(path).map(|x| x.eq_ignore_ascii_case(h)).unwrap_or(false),
        Hash::None => meta.len() > 0,
    }
}

fn allowed_host(url: &str) -> bool {
    const HOSTS: [&str; 13] = [
        "meta.legacyfabric.net",
        "maven.legacyfabric.net",
        "repo.legacyfabric.net",
        "piston-meta.mojang.com",
        "piston-data.mojang.com",
        "launchermeta.mojang.com",
        "launcher.mojang.com",
        "libraries.minecraft.net",
        "resources.download.minecraft.net",
        "meta.fabricmc.net",
        "maven.fabricmc.net",
        "cdn.modrinth.com",
        "api.modrinth.com",
    ];
    url::Url::parse(url)
        .ok()
        .filter(|u| u.scheme() == "https")
        .and_then(|u| u.host_str().map(|h| HOSTS.contains(&h)))
        .unwrap_or(false)
}

/// Download one file to `path`, verifying its hash. Writes to a temporary file first.
pub async fn fetch(d: &Download, progress: Option<&AtomicU64>) -> AppResult<()> {
    if !allowed_host(&d.url) {
        bail!("DOWNLOAD", "Refusing to download from an unexpected host: {}", url::Url::parse(&d.url).ok().and_then(|u| u.host_str().map(str::to_owned)).unwrap_or_default());
    }
    if let Some(parent) = d.path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = part_path(&d.path);
    let mut last_err: Option<AppError> = None;
    for attempt in 0..3u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(600 * attempt as u64)).await;
        }
        match fetch_once(d, &tmp, progress).await {
            Ok(()) => {
                tokio::fs::rename(&tmp, &d.path).await?;
                return Ok(());
            }
            Err(e) => {
                let _ = tokio::fs::remove_file(&tmp).await;
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| AppError::new("DOWNLOAD", "Download failed")))
}

/// `java.exe` → `java.exe.part`. The suffix is appended rather than replacing the extension, so files that differ
/// only by extension (`bin/java.exe` and `bin/java.dll`) never share a temporary file.
fn part_path(path: &Path) -> std::path::PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".part");
    path.with_file_name(name)
}

async fn fetch_once(d: &Download, tmp: &Path, progress: Option<&AtomicU64>) -> AppResult<()> {
    let resp = client().get(&d.url).send().await?;
    if !resp.status().is_success() {
        bail!("DOWNLOAD", "Server returned {} for {}", resp.status(), file_name(&d.path));
    }
    let mut file = tokio::fs::File::create(tmp).await?;
    let mut sha1 = sha1::Sha1::new();
    let mut sha512 = sha2::Sha512::new();
    let mut stream = resp.bytes_stream();
    let mut written: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        match d.hash {
            Hash::Sha1(_) => sha1.update(&chunk),
            Hash::Sha512(_) => sha512.update(&chunk),
            Hash::None => {}
        }
        file.write_all(&chunk).await?;
        written += chunk.len() as u64;
        if let Some(p) = progress {
            p.fetch_add(chunk.len() as u64, Ordering::Relaxed);
        }
    }
    file.flush().await?;
    drop(file);
    if let Some(size) = d.size {
        if written != size {
            bail!("DOWNLOAD", "{} has the wrong size (expected {size}, got {written})", file_name(&d.path));
        }
    }
    let ok = match &d.hash {
        Hash::Sha1(h) => hex::encode(sha1.finalize()).eq_ignore_ascii_case(h),
        Hash::Sha512(h) => hex::encode(sha512.finalize()).eq_ignore_ascii_case(h),
        Hash::None => true,
    };
    if !ok {
        bail!("DOWNLOAD", "{} failed its integrity check", file_name(&d.path));
    }
    Ok(())
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

#[derive(Default)]
pub struct BatchProgress {
    pub files_done: AtomicUsize,
    pub bytes_done: AtomicU64,
    /// Files that were missing or failed their check and were downloaded again.
    pub fetched: AtomicUsize,
}

/// Download many files with bounded concurrency. Files that are already valid are skipped.
pub async fn fetch_all(
    items: Vec<Download>,
    concurrency: usize,
    progress: Arc<BatchProgress>,
) -> AppResult<()> {
    let concurrency = concurrency.clamp(1, 16);
    let results = futures_util::stream::iter(items.into_iter().map(|d| {
        let progress = progress.clone();
        async move {
            let valid = {
                let path = d.path.clone();
                let hash = d.hash.clone();
                let size = d.size;
                tokio::task::spawn_blocking(move || is_valid(&path, &hash, size)).await.unwrap_or(false)
            };
            if !valid {
                fetch(&d, Some(&progress.bytes_done)).await?;
                progress.fetched.fetch_add(1, Ordering::Relaxed);
            } else if let Some(s) = d.size {
                progress.bytes_done.fetch_add(s, Ordering::Relaxed);
            }
            progress.files_done.fetch_add(1, Ordering::Relaxed);
            Ok::<(), AppError>(())
        }
    }))
    .buffer_unordered(concurrency)
    .collect::<Vec<_>>()
    .await;
    for r in results {
        r?;
    }
    Ok(())
}

pub async fn get_json(url: &str) -> AppResult<serde_json::Value> {
    if !allowed_host(url) {
        bail!("DOWNLOAD", "Unexpected metadata host");
    }
    let resp = client().get(url).timeout(Duration::from_secs(30)).send().await?;
    if !resp.status().is_success() {
        bail!("NETWORK", "Server returned {}", resp.status());
    }
    Ok(resp.json().await?)
}

#[cfg(test)]
mod tests {
    use super::part_path;
    use std::path::Path;

    #[test]
    fn temporary_names_never_collide() {
        assert_eq!(part_path(Path::new("rt/bin/java.exe")), Path::new("rt/bin/java.exe.part"));
        assert_ne!(part_path(Path::new("rt/bin/java.exe")), part_path(Path::new("rt/bin/java.dll")));
        assert_eq!(part_path(Path::new("objects/ab/abcdef")), Path::new("objects/ab/abcdef.part"));
    }
}
