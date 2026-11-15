//! Launcher settings (`settings.json`) and global appearance (`appearance.json`, shared with the in-game client).
//! Neither file ever contains credentials.

use crate::bail;
use crate::error::AppResult;
use crate::paths;
use crate::util::fsx;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const THEMES_JSON: &str = include_str!("../../../shared/themes.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LauncherBehaviour {
    /// What the launcher window does after a Save starts: "keep", "minimize" or "hide" (restored when the game exits).
    pub after_launch: String,
    /// Turn off launcher animations and transitions.
    pub reduce_motion: bool,
    /// Page shown when Stabled opens: "home" or "saves".
    pub start_page: String,
    /// Include Minecraft: Java Edition release notes from Mojang in the news panel.
    pub show_minecraft_news: bool,
}

impl Default for LauncherBehaviour {
    fn default() -> Self {
        Self {
            after_launch: "minimize".into(),
            reduce_motion: false,
            start_page: "saves".into(),
            show_minecraft_news: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SaveDefaults {
    pub minecraft_version: String,
    pub memory_mb: u32,
    pub performance_preset: String,
    pub fps_enabled: bool,
    pub motion_blur_enabled: bool,
    pub discord_enabled: bool,
    /// Game mode for new Saves (and Saves that never chose one). CPVP when Stabled is first installed.
    pub game_mode: String,
}

impl Default for SaveDefaults {
    fn default() -> Self {
        Self {
            minecraft_version: crate::minecraft::stack::PRIMARY_VERSION.into(),
            memory_mb: 4096,
            performance_preset: "balanced".into(),
            fps_enabled: true,
            motion_blur_enabled: false,
            discord_enabled: true,
            game_mode: crate::minecraft::stack::default_game_mode().into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Advanced {
    /// Extra JVM arguments appended for every Save. Arguments that would break isolation are rejected.
    pub jvm_args: String,
    /// Parallel downloads while installing a Save.
    pub concurrent_downloads: u32,
}

impl Default for Advanced {
    fn default() -> Self {
        Self { jvm_args: String::new(), concurrent_downloads: 8 }
    }
}

/// Streamer mode: hides personal details in the launcher while streaming or recording. Every feature that shows
/// something personal checks `enabled` plus its own switch, so new features join by adding a switch here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StreamerMode {
    pub enabled: bool,
    /// Minecraft username (header, greeting, account, sign-in).
    pub hide_username: bool,
    /// Skin face next to the username.
    pub hide_avatar: bool,
    /// Folder locations, which include the Windows user name (Settings, logs, messages).
    pub hide_paths: bool,
    /// Usernames, user folders and session details in game logs and crash reports shown by the launcher.
    pub redact_logs: bool,
    /// The computer specifications bar (unless fake specs are on).
    pub hide_specs: bool,
}

impl Default for StreamerMode {
    fn default() -> Self {
        Self { enabled: false, hide_username: true, hide_avatar: true, hide_paths: true, redact_logs: true, hide_specs: false }
    }
}

/// Display-only specifications for the specs bar. Nothing else uses them: memory suggestions and the game still
/// use the real hardware.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FakeSpecs {
    pub enabled: bool,
    pub cpu: String,
    pub gpu: String,
    pub ram_gb: u32,
}

impl Default for FakeSpecs {
    fn default() -> Self {
        Self { enabled: false, cpu: String::new(), gpu: String::new(), ram_gb: 0 }
    }
}

/// Automatic world backups (see `backups.rs`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Backups {
    /// Back up worlds a session changed, after the game has closed.
    pub after_sessions: bool,
    /// Back up every world before Stabled upgrades a Save's game files or mods.
    pub before_upgrades: bool,
    /// Automatic backups kept per world (manual and pre-upgrade backups are kept until deleted).
    pub keep: u32,
}

impl Default for Backups {
    fn default() -> Self {
        Self { after_sessions: true, before_upgrades: true, keep: 5 }
    }
}

/// Current settings format. Older files are migrated in `sanitized`.
const SETTINGS_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Settings format version (0 for files written before versions were recorded).
    pub version: u32,
    pub launcher: LauncherBehaviour,
    pub save_defaults: SaveDefaults,
    pub advanced: Advanced,
    pub streamer: StreamerMode,
    pub fake_specs: FakeSpecs,
    pub backups: Backups,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            launcher: LauncherBehaviour::default(),
            save_defaults: SaveDefaults::default(),
            advanced: Advanced::default(),
            streamer: StreamerMode::default(),
            fake_specs: FakeSpecs::default(),
            backups: Backups::default(),
        }
    }
}

impl Settings {
    pub fn load() -> Settings {
        fsx::read_json::<Settings>(&paths::settings_file()).unwrap_or_default().sanitized()
    }

    pub fn save(&self) -> AppResult<()> {
        fsx::write_json(&paths::settings_file(), self)
    }

    fn sanitized(mut self) -> Settings {
        if self.version < 2 {
            // Version 2 made Saves the default start page. An older file can't tell an explicit "Home" from the old
            // default, so it moves to the new default once; choosing Home again afterwards is kept.
            if self.launcher.start_page == "home" {
                self.launcher.start_page = "saves".into();
            }
        }
        self.version = SETTINGS_VERSION;
        if !["keep", "minimize", "hide"].contains(&self.launcher.after_launch.as_str()) {
            self.launcher.after_launch = "minimize".into();
        }
        if !["home", "saves"].contains(&self.launcher.start_page.as_str()) {
            self.launcher.start_page = "saves".into();
        }
        self.save_defaults.memory_mb = self.save_defaults.memory_mb.clamp(1024, 32768);
        if !["performance", "balanced", "quality", "custom"].contains(&self.save_defaults.performance_preset.as_str()) {
            self.save_defaults.performance_preset = "balanced".into();
        }
        if !crate::minecraft::stack::is_game_mode(&self.save_defaults.game_mode) {
            self.save_defaults.game_mode = crate::minecraft::stack::default_game_mode().into();
        }
        if !crate::minecraft::stack::is_supported(&self.save_defaults.minecraft_version) {
            self.save_defaults.minecraft_version = crate::minecraft::stack::PRIMARY_VERSION.into();
        }
        self.advanced.concurrent_downloads = self.advanced.concurrent_downloads.clamp(1, 16);
        let clean = |s: &str| -> String { s.chars().filter(|c| !c.is_control()).take(80).collect::<String>().trim().to_string() };
        self.fake_specs.cpu = clean(&self.fake_specs.cpu);
        self.fake_specs.gpu = clean(&self.fake_specs.gpu);
        self.fake_specs.ram_gb = self.fake_specs.ram_gb.min(4096);
        self.backups.keep = self.backups.keep.clamp(1, 50);
        self
    }

    pub fn validate(self) -> AppResult<Settings> {
        validate_jvm_args(&self.advanced.jvm_args)?;
        Ok(self.sanitized())
    }
}

/// Extra JVM arguments may tune the JVM, but may not replace what keeps Saves isolated and authenticated.
pub fn validate_jvm_args(args: &str) -> AppResult<Vec<String>> {
    let parts: Vec<String> = args.split_whitespace().map(str::to_owned).collect();
    for p in &parts {
        let lower = p.to_ascii_lowercase();
        if lower == "-cp" || lower == "-classpath" || lower.starts_with("--class-path") || lower.starts_with("-djava.library.path")
            || lower.starts_with("-dstabled.") || lower.starts_with("-javaagent") || lower.starts_with("-xmx") || lower.starts_with("-xms")
        {
            bail!("JVM_ARGS", "\"{p}\" can't be used here. Memory is set per Save; class path, agents and Stabled properties are managed by Stabled.");
        }
    }
    Ok(parts)
}

// ---------- Appearance (shared with the in-game client; schema in docs/CONTRACTS.md) ----------

pub fn builtin_theme_ids() -> Vec<String> {
    let v: Value = serde_json::from_str(THEMES_JSON).expect("themes.json");
    v["themes"].as_array().map(|a| a.iter().filter_map(|t| t["id"].as_str().map(str::to_owned)).collect()).unwrap_or_default()
}

pub fn load_appearance() -> Value {
    let mut v = fsx::read_json_value(&paths::appearance_file());
    normalize_appearance(&mut v);
    v
}

fn normalize_appearance(v: &mut Value) {
    if !v.is_object() {
        *v = serde_json::json!({});
    }
    let ids = builtin_theme_ids();
    let theme = v.get("theme").and_then(Value::as_str).unwrap_or("stabled-night").to_string();
    if !(ids.contains(&theme) || theme == "custom") {
        v["theme"] = "stabled-night".into();
    } else if !v.get("theme").map(Value::is_string).unwrap_or(false) {
        v["theme"] = theme.into();
    }
    let surface = v.get("surface").and_then(Value::as_str).unwrap_or("simple");
    v["surface"] = if surface == "glass" { "glass" } else { "simple" }.into();
    let t = v.get("glassTransparency").and_then(Value::as_f64).unwrap_or(35.0).round().clamp(10.0, 90.0);
    v["glassTransparency"] = (t as i64).into();
    v["version"] = 1.into();
    if !v.get("customBackground").map(|c| c.is_object() || c.is_null()).unwrap_or(false) {
        v["customBackground"] = Value::Null;
    }
}

/// Apply a partial update from the UI. Only the documented keys can change; unknown keys written by the game are kept.
pub fn update_appearance(patch: &Value) -> AppResult<Value> {
    let mut v = load_appearance();
    if let Some(theme) = patch.get("theme").and_then(Value::as_str) {
        if !(builtin_theme_ids().iter().any(|t| t == theme) || theme == "custom") {
            bail!("APPEARANCE", "Unknown theme");
        }
        v["theme"] = theme.into();
    }
    if let Some(surface) = patch.get("surface").and_then(Value::as_str) {
        if surface != "simple" && surface != "glass" {
            bail!("APPEARANCE", "Unknown surface style");
        }
        v["surface"] = surface.into();
    }
    if let Some(t) = patch.get("glassTransparency").and_then(Value::as_f64) {
        v["glassTransparency"] = (t.round().clamp(10.0, 90.0) as i64).into();
    }
    if let Some(bg) = patch.get("customBackground").and_then(Value::as_object) {
        if v["customBackground"].is_object() {
            for key in ["focusX", "focusY"] {
                if let Some(x) = bg.get(key).and_then(Value::as_f64) {
                    v["customBackground"][key] = x.clamp(0.0, 1.0).into();
                }
            }
            if let Some(x) = bg.get("dim").and_then(Value::as_f64) {
                v["customBackground"]["dim"] = x.clamp(0.0, 0.85).into();
            }
        }
    }
    normalize_appearance(&mut v);
    fsx::write_json(&paths::appearance_file(), &v)?;
    Ok(v)
}

/// Replace the single Custom Background slot with a managed copy of the chosen image. `origin` records where it
/// came from: "upload" or "gallery:<category>/<image>" (the UI marks the selected gallery image).
pub fn set_custom_background(source: &std::path::Path, origin: &str) -> AppResult<Value> {
    let png = crate::images::prepare_background(source)?;
    let dir = paths::backgrounds_dir();
    std::fs::create_dir_all(&dir)?;
    fsx::write_atomic(&dir.join("custom.png"), &png)?;
    let mut v = load_appearance();
    let revision = v["customBackground"]["revision"].as_i64().unwrap_or(0) + 1;
    let keep = |k: &str, d: f64| v["customBackground"][k].as_f64().unwrap_or(d);
    // A new image starts centred; the dimming the player chose is kept.
    let bg = serde_json::json!({
        "file": "backgrounds/custom.png",
        "focusX": 0.5,
        "focusY": 0.5,
        "dim": keep("dim", 0.45),
        "revision": revision,
        "source": origin,
    });
    v["customBackground"] = bg;
    v["theme"] = "custom".into();
    normalize_appearance(&mut v);
    fsx::write_json(&paths::appearance_file(), &v)?;
    Ok(v)
}

pub fn reset_custom_background() -> AppResult<Value> {
    let _ = std::fs::remove_file(paths::backgrounds_dir().join("custom.png"));
    let mut v = load_appearance();
    v["customBackground"] = Value::Null;
    if v["theme"] == "custom" {
        v["theme"] = "stabled-night".into();
    }
    fsx::write_json(&paths::appearance_file(), &v)?;
    Ok(v)
}

/// The managed custom background as a data URL, or None if missing/unreadable (the UI then falls back gracefully).
pub fn custom_background_data_url() -> Option<String> {
    let bytes = std::fs::read(paths::backgrounds_dir().join("custom.png")).ok()?;
    image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).ok()?;
    use base64::Engine;
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}
