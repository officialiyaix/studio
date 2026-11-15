//! Saves: isolated Minecraft installations.
//!
//! Layout of `%APPDATA%\Stabled\saves\<id>\` (the id is a UUID and never changes, so renaming is metadata-only):
//! ```text
//! save.json            metadata (name, version, icon, …)
//! icon.png             managed copy of the chosen icon (optional)
//! stabled/             Stabled state: client.json, onboarding.json, managed.json, runtime.json
//! minecraft/           the game directory: mods, config, saves (worlds), resourcepacks, shaderpacks,
//!                      screenshots, logs, options.txt — all private to this Save
//! ```

pub mod client;
pub mod managed;

use crate::bail;
use crate::error::AppResult;
use crate::paths;
use crate::settings::Settings;
use crate::util::fsx;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveMeta {
    #[serde(default = "one")]
    pub version: u32,
    pub id: String,
    pub name: String,
    pub minecraft_version: String,
    pub loader: String,
    pub loader_version: String,
    pub created_at: String,
    #[serde(default)]
    pub last_played_at: Option<String>,
    #[serde(default)]
    pub has_icon: bool,
    #[serde(default)]
    pub icon_revision: u32,
    #[serde(default)]
    pub memory_mb: Option<u32>,
    #[serde(default)]
    pub installed_stack: Option<String>,
    #[serde(default)]
    pub launches: u32,
    /// Set on the temporary test copies made by Find the broken mod: the Save they copy. Test copies are hidden
    /// from the Saves list and never backed up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_of: Option<String>,
    /// Game mode (a set of extra mods, e.g. "cpvp"). None = the default in Settings → Save defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_mode: Option<String>,
}

fn one() -> u32 {
    1
}

pub const GAME_SUBDIRS: [&str; 7] = ["mods", "config", "saves", "resourcepacks", "shaderpacks", "screenshots", "logs"];

impl SaveMeta {
    pub fn root(&self) -> PathBuf {
        root(&self.id)
    }
    pub fn game_dir(&self) -> PathBuf {
        self.root().join("minecraft")
    }
    pub fn mods_dir(&self) -> PathBuf {
        self.game_dir().join("mods")
    }
    pub fn stabled_dir(&self) -> PathBuf {
        self.root().join("stabled")
    }
    pub fn save(&self) -> AppResult<()> {
        fsx::write_json(&self.root().join("save.json"), self)
    }
    /// The game mode in effect: the Save's own, or the default for Saves that never chose one.
    pub fn effective_game_mode(&self) -> String {
        match &self.game_mode {
            Some(m) if crate::minecraft::stack::is_game_mode(m) => m.clone(),
            _ => Settings::load().save_defaults.game_mode,
        }
    }
}

pub fn root(id: &str) -> PathBuf {
    paths::saves_dir().join(id)
}

fn valid_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok()
}

pub fn load(id: &str) -> AppResult<SaveMeta> {
    if !valid_id(id) {
        bail!("SAVE", "Unknown Save");
    }
    let meta: SaveMeta = fsx::read_json(&root(id).join("save.json")).map_err(|_| crate::error::AppError::new("SAVE", "This Save could not be found"))?;
    if meta.id != id {
        bail!("SAVE", "This Save's metadata is inconsistent");
    }
    Ok(meta)
}

pub fn list() -> Vec<SaveMeta> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(paths::saves_dir()) {
        for e in rd.flatten() {
            let id = e.file_name().to_string_lossy().into_owned();
            if let Ok(m) = load(&id) {
                if m.test_of.is_none() {
                    out.push(m);
                }
            }
        }
    }
    out.sort_by(|a, b| b.last_played_at.cmp(&a.last_played_at).then(a.created_at.cmp(&b.created_at)));
    out
}

/// Validates a display name. Names are display-only; folders use the stable id.
pub fn validate_name(name: &str, except: Option<&str>) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        bail!("NAME", "Enter a name for this Save.");
    }
    if name.chars().count() > 40 {
        bail!("NAME", "Use 40 characters or fewer.");
    }
    if name.chars().any(char::is_control) {
        bail!("NAME", "The name contains characters that can't be used.");
    }
    if list().iter().any(|s| s.name.eq_ignore_ascii_case(name) && Some(s.id.as_str()) != except) {
        bail!("NAME", "You already have a Save called \"{name}\".");
    }
    Ok(name.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    pub name: String,
    pub minecraft_version: String,
    /// PNG produced by the launcher's crop/fit step (base64). None → default Stabled icon.
    pub icon_png_base64: Option<String>,
    pub memory_mb: Option<u32>,
    pub performance_preset: Option<String>,
    /// Game mode; None = the default in Settings.
    #[serde(default)]
    pub game_mode: Option<String>,
}

pub fn create(req: CreateRequest, settings: &Settings) -> AppResult<SaveMeta> {
    let name = validate_name(&req.name, None)?;
    let Some(stack) = crate::minecraft::stack::for_version(&req.minecraft_version) else {
        bail!("VERSION", "Choose a Minecraft version supported by the Stabled client.");
    };
    let icon = match &req.icon_png_base64 {
        Some(b64) => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|_| crate::error::AppError::new("IMAGE", "The icon could not be read."))?;
            Some(crate::images::prepare_icon(&bytes)?)
        }
        None => None,
    };
    let meta = SaveMeta {
        version: 1,
        id: uuid::Uuid::new_v4().to_string(),
        name,
        minecraft_version: stack.minecraft.clone(),
        loader: stack.loader.id.clone(),
        loader_version: stack.loader.version.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_played_at: None,
        has_icon: icon.is_some(),
        icon_revision: 1,
        memory_mb: req.memory_mb.map(|m| m.clamp(1024, 32768)),
        installed_stack: None,
        launches: 0,
        test_of: None,
        game_mode: Some(
            req.game_mode.clone().filter(|m| crate::minecraft::stack::is_game_mode(m)).unwrap_or_else(|| settings.save_defaults.game_mode.clone()),
        ),
    };
    let root = meta.root();
    for sub in GAME_SUBDIRS {
        std::fs::create_dir_all(meta.game_dir().join(sub))?;
    }
    std::fs::create_dir_all(meta.stabled_dir())?;
    if let Some(png) = icon {
        fsx::write_atomic(&root.join("icon.png"), &png)?;
    }
    let mut defaults = settings.save_defaults.clone();
    if let Some(p) = req.performance_preset {
        defaults.performance_preset = p;
    }
    client::write_defaults(&meta, &defaults)?;
    meta.save()?;
    crate::log_info!("Created Save {} ({})", meta.id, meta.minecraft_version);
    Ok(meta)
}

pub fn rename(id: &str, name: &str) -> AppResult<SaveMeta> {
    let mut meta = load(id)?;
    meta.name = validate_name(name, Some(id))?;
    meta.save()?;
    Ok(meta)
}

pub fn set_icon(id: &str, png_base64: Option<String>) -> AppResult<SaveMeta> {
    let mut meta = load(id)?;
    match png_base64 {
        Some(b64) => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|_| crate::error::AppError::new("IMAGE", "The icon could not be read."))?;
            let png = crate::images::prepare_icon(&bytes)?;
            fsx::write_atomic(&meta.root().join("icon.png"), &png)?;
            meta.has_icon = true;
        }
        None => {
            let _ = std::fs::remove_file(meta.root().join("icon.png"));
            meta.has_icon = false;
        }
    }
    meta.icon_revision += 1;
    meta.save()?;
    Ok(meta)
}

pub fn icon_data_url(meta: &SaveMeta) -> Option<String> {
    if !meta.has_icon {
        return None;
    }
    let bytes = std::fs::read(meta.root().join("icon.png")).ok()?;
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// Full copy (worlds, mods, settings) under a new id. Runtime and lock files are not copied.
pub fn duplicate(id: &str, name: Option<String>) -> AppResult<SaveMeta> {
    let src = load(id)?;
    let base = name.unwrap_or_else(|| format!("{} (copy)", src.name));
    let mut candidate = base.clone();
    let mut n = 2;
    while validate_name(&candidate, None).is_err() {
        if n > 50 {
            return Err(validate_name(&candidate, None).unwrap_err());
        }
        candidate = format!("{base} {n}");
        n += 1;
    }
    let mut meta = src.clone();
    meta.id = uuid::Uuid::new_v4().to_string();
    meta.name = candidate;
    meta.created_at = chrono::Utc::now().to_rfc3339();
    meta.last_played_at = None;
    meta.launches = 0;
    fsx::copy_dir(&src.root(), &meta.root())?;
    let _ = std::fs::remove_file(meta.stabled_dir().join("runtime.json"));
    // World backups belong to the original Save.
    let _ = std::fs::remove_dir_all(meta.root().join("backups"));
    meta.save()?;
    Ok(meta)
}

/// A hidden copy of a Save for Find the broken mod: settings, mods, configs and packs, plus the worlds only when
/// asked (needed when the problem happens inside a world). The original Save is never changed.
pub fn create_test_copy(id: &str, with_worlds: bool) -> AppResult<SaveMeta> {
    let src = load(id)?;
    // One test copy per Save at a time.
    for e in std::fs::read_dir(paths::saves_dir()).into_iter().flatten().flatten() {
        if let Ok(m) = load(&e.file_name().to_string_lossy()) {
            if m.test_of.as_deref() == Some(id) {
                delete_test_copy(&m.id)?;
            }
        }
    }
    let mut meta = src.clone();
    meta.id = uuid::Uuid::new_v4().to_string();
    meta.name = format!("{} (test copy)", src.name);
    meta.created_at = chrono::Utc::now().to_rfc3339();
    meta.last_played_at = None;
    meta.launches = 0;
    meta.test_of = Some(src.id.clone());
    std::fs::create_dir_all(meta.game_dir())?;
    fsx::copy_dir(&src.stabled_dir(), &meta.stabled_dir())?;
    let _ = std::fs::remove_file(meta.stabled_dir().join("runtime.json"));
    let skip: &[&str] = if with_worlds { &["logs", "crash-reports", "screenshots"] } else { &["saves", "logs", "crash-reports", "screenshots"] };
    for e in std::fs::read_dir(src.game_dir())?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if skip.contains(&name.as_str()) || name.starts_with("hs_err_pid") {
            continue;
        }
        let to = meta.game_dir().join(&name);
        if e.path().is_dir() {
            fsx::copy_dir(&e.path(), &to)?;
        } else {
            std::fs::copy(e.path(), &to)?;
        }
    }
    if let Ok(icon) = std::fs::read(src.root().join("icon.png")) {
        let _ = std::fs::write(meta.root().join("icon.png"), icon);
    }
    meta.save()?;
    crate::log_info!("Created test copy {} of Save {}", meta.id, src.id);
    Ok(meta)
}

/// Deletes a test copy permanently (it only ever held copies).
pub fn delete_test_copy(id: &str) -> AppResult<()> {
    let meta = load(id)?;
    if meta.test_of.is_none() {
        bail!("SAVE", "That isn't a test copy");
    }
    let root = meta.root();
    if !root.starts_with(paths::saves_dir()) {
        bail!("SAVE", "Refusing to delete outside the Saves folder");
    }
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// Moves the whole Save folder (including its worlds) to the Recycle Bin, so it can still be recovered from there.
pub fn delete(id: &str) -> AppResult<()> {
    let meta = load(id)?;
    let root = meta.root();
    if !root.starts_with(paths::saves_dir()) {
        bail!("SAVE", "Refusing to delete outside the Saves folder");
    }
    trash::delete(&root).map_err(|e| crate::error::AppError::new("DELETE", format!("Could not move the Save to the Recycle Bin: {e}")))?;
    crate::log_info!("Moved Save {} to the Recycle Bin", id);
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSummary {
    pub worlds: usize,
    pub screenshots: usize,
    pub mods: usize,
    pub size_bytes: u64,
}

pub fn summary(meta: &SaveMeta) -> SaveSummary {
    let count_files = |p: PathBuf| std::fs::read_dir(p).map(|r| r.flatten().filter(|e| e.path().is_file()).count()).unwrap_or(0);
    SaveSummary {
        worlds: fsx::count_dirs(&meta.game_dir().join("saves")),
        screenshots: count_files(meta.game_dir().join("screenshots")),
        mods: count_files(meta.mods_dir()),
        size_bytes: fsx::dir_size(&meta.root()),
    }
}
