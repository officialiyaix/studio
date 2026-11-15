//! Data locations.
//!
//! * `%APPDATA%\Stabled` (roaming) — settings, appearance, Saves (each with its own game directory) and the
//!   encrypted account file. This mirrors where the official launcher keeps `.minecraft`.
//! * `%LOCALAPPDATA%\Stabled\shared` (local) — immutable, content-addressed downloads shared by all Saves:
//!   game jars, libraries, assets, Java runtimes and mod files. Never roamed, never mutated per Save.

use std::path::PathBuf;
use std::sync::OnceLock;

static ROOT: OnceLock<PathBuf> = OnceLock::new();
static SHARED: OnceLock<PathBuf> = OnceLock::new();

pub fn root() -> &'static PathBuf {
    ROOT.get_or_init(|| {
        if let Ok(custom) = std::env::var("STABLED_DATA_DIR") {
            return PathBuf::from(custom);
        }
        dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("Stabled")
    })
}

pub fn shared() -> &'static PathBuf {
    SHARED.get_or_init(|| {
        if let Ok(custom) = std::env::var("STABLED_DATA_DIR") {
            return PathBuf::from(custom).join("shared");
        }
        dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("Stabled").join("shared")
    })
}

pub fn saves_dir() -> PathBuf {
    root().join("saves")
}

pub fn settings_file() -> PathBuf {
    root().join("settings.json")
}

pub fn appearance_file() -> PathBuf {
    root().join("appearance.json")
}

pub fn backgrounds_dir() -> PathBuf {
    root().join("backgrounds")
}

pub fn secure_dir() -> PathBuf {
    root().join("secure")
}

pub fn cache_dir() -> PathBuf {
    root().join("cache")
}

pub fn logs_dir() -> PathBuf {
    root().join("logs")
}

pub fn optimiser_dir() -> PathBuf {
    root().join("optimiser")
}

pub fn versions_dir() -> PathBuf {
    shared().join("versions")
}

pub fn libraries_dir() -> PathBuf {
    shared().join("libraries")
}

pub fn assets_dir() -> PathBuf {
    shared().join("assets")
}

pub fn runtimes_dir() -> PathBuf {
    shared().join("runtimes")
}

pub fn mod_cache_dir() -> PathBuf {
    shared().join("mods")
}

pub fn natives_dir() -> PathBuf {
    shared().join("natives")
}

pub fn ensure_base_dirs() -> std::io::Result<()> {
    for dir in [root().clone(), saves_dir(), backgrounds_dir(), secure_dir(), cache_dir(), logs_dir(), shared().clone()] {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}
