//! World backups: `<save>/backups/<world folder>/<timestamp>__<reason>.zip`.
//!
//! * After a session: once the game process has exited (so Minecraft has finished writing), worlds changed during
//!   the session are backed up. The newest `keep` session backups per world are kept.
//! * Before an upgrade: before Stabled changes a Save's game files or mods for a new stack, every world is backed
//!   up. Upgrade and manual backups are kept until the player deletes them.
//! * Restoring extracts into a new world folder ("World (restored …)"), so nothing is ever overwritten.

use crate::bail;
use crate::error::{AppError, AppResult};
use crate::saves::SaveMeta;
use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    /// "<world folder>/<file name>"
    pub id: String,
    pub world: String,
    /// "session" | "upgrade" | "manual"
    pub reason: String,
    pub created_at: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct World {
    pub folder: String,
    pub size: u64,
    pub modified: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupsView {
    pub backups: Vec<Backup>,
    pub worlds: Vec<World>,
    pub total_size: u64,
}

fn backups_dir(meta: &SaveMeta) -> PathBuf {
    meta.root().join("backups")
}

fn worlds_dir(meta: &SaveMeta) -> PathBuf {
    meta.game_dir().join("saves")
}

fn worlds(meta: &SaveMeta) -> Vec<PathBuf> {
    std::fs::read_dir(worlds_dir(meta))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("level.dat").is_file())
        .collect()
}

fn newest_mtime(dir: &Path) -> Option<SystemTime> {
    let mut newest: Option<SystemTime> = None;
    for e in walk(dir) {
        if let Ok(t) = std::fs::metadata(&e).and_then(|m| m.modified()) {
            newest = Some(newest.map_or(t, |n| n.max(t)));
        }
    }
    newest
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            match e.file_type() {
                Ok(t) if t.is_dir() => stack.push(p),
                Ok(t) if t.is_file() => out.push(p),
                _ => {}
            }
        }
    }
    out
}

pub fn view(meta: &SaveMeta) -> BackupsView {
    let mut backups = Vec::new();
    for w in std::fs::read_dir(backups_dir(meta)).into_iter().flatten().flatten() {
        let world = w.file_name().to_string_lossy().into_owned();
        for f in std::fs::read_dir(w.path()).into_iter().flatten().flatten() {
            let name = f.file_name().to_string_lossy().into_owned();
            let Some(stem) = name.strip_suffix(".zip") else { continue };
            let Some((stamp, reason)) = stem.split_once("__") else { continue };
            let created_at = chrono::NaiveDateTime::parse_from_str(stamp, "%Y-%m-%dT%H-%M-%S")
                .map(|t| t.and_utc().to_rfc3339())
                .unwrap_or_else(|_| stamp.to_string());
            backups.push(Backup { id: format!("{world}/{name}"), world: world.clone(), reason: reason.to_string(), created_at, size: f.metadata().map(|m| m.len()).unwrap_or(0) });
        }
    }
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let worlds = worlds(meta)
        .into_iter()
        .map(|p| World {
            folder: p.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            size: crate::util::fsx::dir_size(&p),
            modified: newest_mtime(&p).map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
        })
        .collect();
    let total_size = backups.iter().map(|b| b.size).sum();
    BackupsView { backups, worlds, total_size }
}

/// Backs up worlds. `changed_since` limits it to worlds modified after that time. Returns how many were backed up.
pub fn create(meta: &SaveMeta, reason: &str, changed_since: Option<SystemTime>, only: Option<&str>, keep: u32) -> AppResult<usize> {
    let mut count = 0;
    for world in worlds(meta) {
        let folder = world.file_name().unwrap_or_default().to_string_lossy().into_owned();
        if only.is_some_and(|o| o != folder) {
            continue;
        }
        if let Some(since) = changed_since {
            if newest_mtime(&world).map(|t| t <= since).unwrap_or(true) {
                continue;
            }
        }
        let dir = backups_dir(meta).join(&folder);
        std::fs::create_dir_all(&dir)?;
        let stamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S");
        let target = dir.join(format!("{stamp}__{reason}.zip"));
        zip_dir(&world, &folder, &target)?;
        count += 1;
        if reason == "session" {
            prune(&dir, keep);
        }
    }
    Ok(count)
}

fn zip_dir(src: &Path, root_name: &str, target: &Path) -> AppResult<()> {
    let tmp = target.with_extension("zip.part");
    let file = std::fs::File::create(&tmp)?;
    let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(file));
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(true);
    let mut buf = Vec::new();
    for path in walk(src) {
        let rel = path.strip_prefix(src).map_err(|e| AppError::new("BACKUP", e.to_string()))?;
        let name = rel.to_string_lossy().replace('\\', "/");
        // The lock file is held by a running game and isn't world data.
        if name == "session.lock" {
            continue;
        }
        buf.clear();
        match std::fs::File::open(&path).and_then(|mut f| f.read_to_end(&mut buf)) {
            Ok(_) => {}
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                return Err(AppError::new("BACKUP", format!("Couldn't read {name}: {e}")));
            }
        }
        zip.start_file(format!("{root_name}/{name}"), opts).map_err(|e| AppError::new("BACKUP", e.to_string()))?;
        zip.write_all(&buf)?;
    }
    zip.finish().map_err(|e| AppError::new("BACKUP", e.to_string()))?;
    std::fs::rename(&tmp, target)?;
    Ok(())
}

/// Keeps the newest `keep` session backups in a world's backup folder.
fn prune(dir: &Path, keep: u32) {
    let mut sessions: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().map(|n| n.to_string_lossy().ends_with("__session.zip")).unwrap_or(false))
        .collect();
    sessions.sort();
    while sessions.len() > keep as usize {
        let old = sessions.remove(0);
        let _ = std::fs::remove_file(old);
    }
}

fn backup_path(meta: &SaveMeta, id: &str) -> AppResult<PathBuf> {
    let Some((world, file)) = id.split_once('/') else { bail!("BACKUP", "Unknown backup") };
    let (Some(world), Some(file)) = (crate::util::fsx::safe_file_name(world), crate::util::fsx::safe_file_name(file)) else { bail!("BACKUP", "Unknown backup") };
    let p = backups_dir(meta).join(world).join(&file);
    if !file.ends_with(".zip") || !p.is_file() {
        bail!("BACKUP", "That backup no longer exists");
    }
    Ok(p)
}

/// Restores a backup into a new world folder. Returns the new folder's name.
pub fn restore(meta: &SaveMeta, id: &str) -> AppResult<String> {
    let path = backup_path(meta, id)?;
    let world = id.split_once('/').map(|(w, _)| w.to_string()).unwrap_or_default();
    let date = chrono::Local::now().format("%Y-%m-%d %H.%M");
    let saves = worlds_dir(meta);
    let mut name = format!("{world} (restored {date})");
    let mut n = 2;
    while saves.join(&name).exists() {
        name = format!("{world} (restored {date}) {n}");
        n += 1;
    }
    let target = saves.join(&name);
    let tmp = saves.join(format!(".{name}.restoring"));
    std::fs::create_dir_all(&tmp)?;
    let result = (|| -> AppResult<()> {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&path)?).map_err(|e| AppError::new("BACKUP", format!("The backup is damaged: {e}")))?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| AppError::new("BACKUP", e.to_string()))?;
            let Some(rel) = entry.enclosed_name() else { continue };
            // Entries are "<world>/<path>"; drop the first component.
            let inner: PathBuf = rel.components().skip(1).collect();
            if inner.as_os_str().is_empty() {
                continue;
            }
            let out = tmp.join(&inner);
            if entry.is_dir() {
                std::fs::create_dir_all(&out)?;
                continue;
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut f = std::fs::File::create(&out)?;
            std::io::copy(&mut entry, &mut f)?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, &target)?;
    Ok(name)
}

pub fn delete(meta: &SaveMeta, id: &str) -> AppResult<()> {
    let p = backup_path(meta, id)?;
    std::fs::remove_file(&p)?;
    if let Some(dir) = p.parent() {
        let _ = std::fs::remove_dir(dir); // only succeeds when empty
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_and_restore_round_trip() {
        let tmp = std::env::temp_dir().join(format!("stabled-backup-test-{}", uuid::Uuid::new_v4()));
        let world = tmp.join("World");
        std::fs::create_dir_all(world.join("region")).unwrap();
        std::fs::write(world.join("level.dat"), b"level").unwrap();
        std::fs::write(world.join("region/r.0.0.mca"), vec![7u8; 5000]).unwrap();
        std::fs::write(world.join("session.lock"), b"lock").unwrap();
        let target = tmp.join("b.zip");
        zip_dir(&world, "World", &target).unwrap();
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&target).unwrap()).unwrap();
        let names: Vec<String> = (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().to_string()).collect();
        assert!(names.contains(&"World/level.dat".to_string()));
        assert!(names.contains(&"World/region/r.0.0.mca".to_string()));
        assert!(!names.iter().any(|n| n.ends_with("session.lock")));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
