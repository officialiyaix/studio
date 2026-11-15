//! Stabled-managed components inside a Save's `mods` folder (`stabled/managed.json`).
//!
//! Rules:
//! * The Stabled client and Fabric API are required: they are restored if missing or disabled and can't be removed.
//! * Performance mods are managed but optional: the user may disable or remove them, and Stabled then respects that.
//! * A file Stabled did not install is never overwritten. If the user already has a mod with the same id, Stabled
//!   keeps the user's file and reports that it skipped its own copy.
//! * Managed files are only replaced when they still match the hash Stabled installed (i.e. unmodified).

use super::SaveMeta;
use crate::bail;
use crate::error::AppResult;
use crate::minecraft::stack::{self, Stack};
use crate::paths;
use crate::util::download::{self, Download, Hash};
use crate::util::fsx;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub mod_id: String,
    pub title: String,
    pub file: String,
    pub sha512: String,
    pub version: String,
    pub required: bool,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub disabled_by_user: bool,
    #[serde(default)]
    pub removed_by_user: bool,
    #[serde(default)]
    pub beta: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ManagedState {
    #[serde(default)]
    pub components: Vec<Component>,
    /// Human-readable notes about components Stabled deliberately did not install.
    #[serde(default)]
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedInfo {
    pub title: String,
    pub required: bool,
    pub purpose: String,
    pub beta: bool,
    pub modified: bool,
}

static BUNDLED_CLIENTS: OnceLock<PathBuf> = OnceLock::new();

/// Set at startup to the folder holding the Stabled client builds shipped with the launcher (one per version).
pub fn set_bundled_clients_dir(path: PathBuf) {
    let _ = BUNDLED_CLIENTS.set(path);
}

/// The Stabled client build for a Minecraft version, if this launcher build includes it.
pub fn bundled_client(minecraft: &str) -> Option<PathBuf> {
    BUNDLED_CLIENTS.get().map(|d| d.join(stack::bundled_jar_name(minecraft))).filter(|p| p.is_file())
}

/// Minecraft versions for which a Stabled client build is bundled.
pub fn bundled_versions() -> Vec<String> {
    stack::all().iter().filter(|s| bundled_client(&s.minecraft).is_some()).map(|s| s.minecraft.clone()).collect()
}

fn file(meta: &SaveMeta) -> PathBuf {
    meta.stabled_dir().join("managed.json")
}

pub fn load(meta: &SaveMeta) -> ManagedState {
    fsx::read_json(&file(meta)).unwrap_or_default()
}

fn store(meta: &SaveMeta, s: &ManagedState) -> AppResult<()> {
    fsx::write_json(&file(meta), s)
}

pub fn info_for(state: &ManagedState, file: &str) -> Option<ManagedInfo> {
    state.components.iter().find(|c| c.file == file).map(|c| ManagedInfo {
        title: c.title.clone(),
        required: c.required,
        purpose: c.purpose.clone(),
        beta: c.beta,
        modified: false,
    })
}

pub fn is_required(state: &ManagedState, file: &str) -> bool {
    state.components.iter().any(|c| c.file == file && c.required)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovedComponent {
    pub mod_id: String,
    pub title: String,
}

pub fn removed(state: &ManagedState) -> Vec<RemovedComponent> {
    state
        .components
        .iter()
        .filter(|c| c.removed_by_user && !c.required)
        .map(|c| RemovedComponent { mod_id: c.mod_id.clone(), title: c.title.clone() })
        .collect()
}

pub fn skipped(state: &ManagedState) -> Vec<String> {
    state.skipped.clone()
}

pub fn note_enabled(meta: &SaveMeta, file: &str, enabled: bool) {
    let mut s = load(meta);
    if let Some(c) = s.components.iter_mut().find(|c| c.file == file) {
        c.disabled_by_user = !enabled;
        let _ = store(meta, &s);
    }
}

pub fn note_removed(meta: &SaveMeta, file: &str) {
    let mut s = load(meta);
    if let Some(c) = s.components.iter_mut().find(|c| c.file == file) {
        c.removed_by_user = true;
        let _ = store(meta, &s);
    }
}

/// Clears "removed by user" so the next sync reinstalls an optional component.
pub fn restore(meta: &SaveMeta, mod_id: &str) -> AppResult<()> {
    let mut s = load(meta);
    match s.components.iter_mut().find(|c| c.mod_id == mod_id) {
        Some(c) => {
            c.removed_by_user = false;
            c.disabled_by_user = false;
            store(meta, &s)
        }
        None => bail!("MODS", "Unknown component"),
    }
}

struct Desired {
    mod_id: String,
    title: String,
    purpose: String,
    file: String,
    sha512: String,
    version: String,
    required: bool,
    beta: bool,
    source: Source,
}

enum Source {
    Bundled(PathBuf),
    Remote { url: String, size: u64 },
}

fn desired(stack: &Stack) -> AppResult<Vec<Desired>> {
    let Some(client) = bundled_client(&stack.minecraft) else {
        bail!("CLIENT_MISSING", "This build of Stabled doesn't include the Stabled client for Minecraft {}, so this Save can't be launched. Rebuild the launcher with `npm run bundle-client` (see README).", stack.minecraft);
    };
    let bytes = std::fs::read(&client)?;
    let mut out = vec![Desired {
        mod_id: stack::STABLED_MOD_ID.into(),
        title: "Stabled Client".into(),
        purpose: "Stabled menus, settings, FPS display, motion blur and Discord Presence".into(),
        file: stack::STABLED_JAR_NAME.into(),
        sha512: download::bytes_sha512(&bytes),
        version: env!("CARGO_PKG_VERSION").into(),
        required: true,
        beta: false,
        source: Source::Bundled(client),
    }];
    for m in &stack.mods {
        out.push(Desired {
            mod_id: m.mod_id.clone(),
            title: m.title.clone(),
            purpose: m.purpose.clone(),
            file: m.filename.clone(),
            sha512: m.sha512.clone(),
            version: m.version_number.clone(),
            required: m.required,
            beta: m.version_type != "release",
            source: Source::Remote { url: m.url.clone(), size: m.size },
        });
    }
    Ok(out)
}

pub async fn sync(meta: &SaveMeta, stack: &Stack, concurrency: usize) -> AppResult<()> {
    let wanted = desired(stack)?;
    // Download remote components into the shared, content-addressed cache first.
    let cache = paths::mod_cache_dir();
    let downloads: Vec<Download> = wanted
        .iter()
        .filter_map(|d| match &d.source {
            Source::Remote { url, size } => Some(Download {
                url: url.clone(),
                path: cache.join(format!("{}.jar", &d.sha512[..32])),
                hash: Hash::Sha512(d.sha512.clone()),
                size: Some(*size),
            }),
            Source::Bundled(_) => None,
        })
        .collect();
    let progress = std::sync::Arc::new(download::BatchProgress::default());
    download::fetch_all(downloads, concurrency, progress).await?;

    let mods_dir = meta.mods_dir();
    std::fs::create_dir_all(&mods_dir)?;
    let mut state = load(meta);
    state.skipped.clear();

    // Mod ids of files the user put here themselves (not recorded as managed).
    let mut user_ids: Vec<(String, String)> = Vec::new();
    for e in std::fs::read_dir(&mods_dir)?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let base = name.trim_end_matches(".disabled").to_string();
        if !base.ends_with(".jar") || state.components.iter().any(|c| c.file == base) {
            continue;
        }
        if let Some(id) = crate::mods::read_mod_id(&e.path()) {
            user_ids.push((id, name));
        }
    }

    for d in &wanted {
        let enabled_path = mods_dir.join(&d.file);
        let disabled_path = mods_dir.join(format!("{}.disabled", d.file));
        let existing = state.components.iter().position(|c| c.mod_id == d.mod_id);

        if let Some((_, user_file)) = user_ids.iter().find(|(id, _)| *id == d.mod_id) {
            if d.mod_id == stack::STABLED_MOD_ID {
                bail!("PROTECTED", "Another file in this Save ({user_file}) claims to be the Stabled client. Remove it from the mods folder to continue.");
            }
            state.skipped.push(format!("Using your own {user_file} instead of the {} {} that Stabled would install.", d.title, d.version));
            continue;
        }

        if let Some(i) = existing {
            let rec = state.components[i].clone();
            if rec.removed_by_user && !d.required {
                continue;
            }
            // Old managed file for this component (possibly an earlier version with another file name).
            let old_enabled = mods_dir.join(&rec.file);
            let old_disabled = mods_dir.join(format!("{}.disabled", rec.file));
            let (old_path, was_disabled) = if old_enabled.is_file() {
                (Some(old_enabled), false)
            } else if old_disabled.is_file() {
                (Some(old_disabled), true)
            } else {
                (None, false)
            };
            if let Some(p) = &old_path {
                let current = download::file_sha512(p).unwrap_or_default();
                let unmodified = current.eq_ignore_ascii_case(&rec.sha512);
                let up_to_date = current.eq_ignore_ascii_case(&d.sha512) && rec.file == d.file;
                if up_to_date {
                    if was_disabled && d.required {
                        std::fs::rename(p, &enabled_path)?;
                    }
                    state.components[i].disabled_by_user = was_disabled && !d.required;
                    continue;
                }
                if !unmodified && d.mod_id != stack::STABLED_MOD_ID {
                    state.skipped.push(format!("{} was changed outside Stabled, so it was left as it is.", rec.file));
                    continue;
                }
                std::fs::remove_file(p)?;
            }
            let disabled = rec.disabled_by_user && !d.required;
            install_file(d, &cache, if disabled { &disabled_path } else { &enabled_path })?;
            state.components[i] = component(d, disabled);
        } else {
            if enabled_path.exists() || disabled_path.exists() {
                state.skipped.push(format!("A file named {} already exists, so Stabled did not replace it.", d.file));
                continue;
            }
            install_file(d, &cache, &enabled_path)?;
            state.components.push(component(d, false));
        }
    }
    // Components no longer part of the stack stay in place but are no longer tracked as managed.
    state.components.retain(|c| wanted.iter().any(|d| d.mod_id == c.mod_id));
    store(meta, &state)
}

fn component(d: &Desired, disabled: bool) -> Component {
    Component {
        mod_id: d.mod_id.clone(),
        title: d.title.clone(),
        file: d.file.clone(),
        sha512: d.sha512.clone(),
        version: d.version.clone(),
        required: d.required,
        purpose: d.purpose.clone(),
        disabled_by_user: disabled,
        removed_by_user: false,
        beta: d.beta,
    }
}

fn install_file(d: &Desired, cache: &std::path::Path, target: &std::path::Path) -> AppResult<()> {
    let src = match &d.source {
        Source::Bundled(p) => p.clone(),
        Source::Remote { .. } => cache.join(format!("{}.jar", &d.sha512[..32])),
    };
    let bytes = std::fs::read(&src)?;
    if !download::bytes_sha512(&bytes).eq_ignore_ascii_case(&d.sha512) {
        bail!("DOWNLOAD", "{} failed its integrity check", d.title);
    }
    fsx::write_atomic(target, &bytes)
}
