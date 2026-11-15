//! Tauri commands: the only interface between the UI and the system. Every command validates its input; secrets
//! never cross this boundary.

use crate::auth::AuthStatus;
use crate::bail;
use crate::error::{AppError, AppResult};
use crate::minecraft::{install, launch, stack};
use crate::saves::{self, client, managed, SaveMeta};
use crate::settings::{self, Settings};
use crate::state::{AppState, RunPhase};
use crate::{images, mods, news, optimiser, paths, system};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

type S<'a> = State<'a, Arc<AppState>>;

// ---------------- App ----------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    platform: String,
    is_windows: bool,
    config: crate::config::ConfigStatus,
    data_dir: String,
    shared_dir: String,
    client_bundled: bool,
    supported_versions: Vec<String>,
    bundled_client_versions: Vec<String>,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        platform: std::env::consts::OS.into(),
        is_windows: cfg!(windows),
        config: crate::config::status(),
        data_dir: paths::root().display().to_string(),
        shared_dir: paths::shared().display().to_string(),
        client_bundled: !managed::bundled_versions().is_empty(),
        supported_versions: stack::all().iter().map(|s| s.minecraft.clone()).collect(),
        bundled_client_versions: managed::bundled_versions(),
    }
}

#[tauri::command]
pub fn system_static() -> system::StaticInfo {
    system::static_info()
}

#[tauri::command]
pub fn system_dynamic() -> system::DynamicInfo {
    system::dynamic_info()
}

#[tauri::command]
pub fn settings_get(state: S) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn settings_set(state: S, settings: Settings) -> AppResult<Settings> {
    let validated = settings.validate()?;
    validated.save()?;
    *state.settings.lock().unwrap() = validated.clone();
    Ok(validated)
}

// ---------------- Appearance ----------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceView {
    appearance: Value,
    custom_background: Option<String>,
    custom_background_error: Option<String>,
    themes: Value,
}

fn appearance_view(appearance: Value) -> AppearanceView {
    let has_record = appearance["customBackground"].is_object();
    let data = if has_record { settings::custom_background_data_url() } else { None };
    AppearanceView {
        custom_background_error: (has_record && data.is_none())
            .then(|| "The custom background image couldn't be loaded. Choose it again or reset it.".to_string()),
        custom_background: data,
        appearance,
        themes: serde_json::from_str(settings::THEMES_JSON).unwrap_or(Value::Null),
    }
}

#[tauri::command]
pub fn appearance_get() -> AppearanceView {
    appearance_view(settings::load_appearance())
}

#[tauri::command]
pub fn appearance_set(patch: Value) -> AppResult<AppearanceView> {
    Ok(appearance_view(settings::update_appearance(&patch)?))
}

#[tauri::command]
pub async fn appearance_set_background(path: String) -> AppResult<AppearanceView> {
    let v = tauri::async_runtime::spawn_blocking(move || settings::set_custom_background(std::path::Path::new(&path)))
        .await
        .map_err(|e| AppError::new("IMAGE", e.to_string()))??;
    Ok(appearance_view(v))
}

#[tauri::command]
pub fn appearance_reset_background() -> AppResult<AppearanceView> {
    Ok(appearance_view(settings::reset_custom_background()?))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInfo {
    data_url: String,
    width: u32,
    height: u32,
}

/// Validates an image the user picked and returns it for the crop/fit preview.
#[tauri::command]
pub async fn image_inspect(path: String) -> AppResult<ImageInfo> {
    tauri::async_runtime::spawn_blocking(move || {
        use base64::Engine;
        let (bytes, mime, width, height) = images::inspect(std::path::Path::new(&path))?;
        Ok(ImageInfo { data_url: format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)), width, height })
    })
    .await
    .map_err(|e| AppError::new("IMAGE", e.to_string()))?
}

// ---------------- Versions & Saves ----------------

#[tauri::command]
pub async fn versions_list() -> crate::minecraft::manifest::VersionList {
    crate::minecraft::manifest::list().await
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaveView {
    #[serde(flatten)]
    meta: SaveMeta,
    icon: Option<String>,
    /// "not_installed" | "ready" | "downloading" | "launching" | "running" | "error"
    status: String,
    phase: Option<RunPhase>,
    /// The Stabled client supports this Minecraft version.
    supported: bool,
    /// This launcher build includes the Stabled client jar for the Save's Minecraft version.
    client_bundled: bool,
}

fn view(state: &AppState, meta: SaveMeta) -> SaveView {
    let phase = state.phase(&meta.id);
    let status = match &phase {
        Some(RunPhase::Downloading { .. }) => "downloading",
        Some(RunPhase::Launching) => "launching",
        Some(RunPhase::Running { .. }) => "running",
        Some(RunPhase::Error { .. }) => "error",
        None if install::is_installed(&meta) => "ready",
        None => "not_installed",
    };
    SaveView {
        icon: saves::icon_data_url(&meta),
        supported: stack::is_supported(&meta.minecraft_version),
        client_bundled: managed::bundled_client(&meta.minecraft_version).is_some(),
        status: status.into(),
        phase,
        meta,
    }
}

fn emit_save(app: &AppHandle, state: &AppState, id: &str) {
    if let Ok(meta) = saves::load(id) {
        let _ = app.emit("save-updated", view(state, meta));
    }
}

fn ensure_idle(state: &AppState, id: &str, what: &str) -> AppResult<()> {
    if state.is_busy(id) {
        bail!("LOCKED", "{what} isn't possible while this Save is running or installing.");
    }
    Ok(())
}

#[tauri::command]
pub fn saves_list(state: S) -> Vec<SaveView> {
    saves::list().into_iter().map(|m| view(&state, m)).collect()
}

#[tauri::command]
pub fn save_get(state: S, id: String) -> AppResult<SaveView> {
    Ok(view(&state, saves::load(&id)?))
}

#[tauri::command]
pub fn save_validate_name(name: String, except: Option<String>) -> AppResult<String> {
    saves::validate_name(&name, except.as_deref())
}

#[tauri::command]
pub async fn save_create(state: S<'_>, request: saves::CreateRequest) -> AppResult<SaveView> {
    let settings = state.settings.lock().unwrap().clone();
    let meta = tauri::async_runtime::spawn_blocking(move || saves::create(request, &settings))
        .await
        .map_err(|e| AppError::new("SAVE", e.to_string()))??;
    Ok(view(&state, meta))
}

#[tauri::command]
pub fn save_rename(app: AppHandle, state: S, id: String, name: String) -> AppResult<SaveView> {
    let meta = saves::rename(&id, &name)?;
    emit_save(&app, &state, &id);
    Ok(view(&state, meta))
}

#[tauri::command]
pub async fn save_set_icon(state: S<'_>, id: String, png_base64: Option<String>) -> AppResult<SaveView> {
    let meta = tauri::async_runtime::spawn_blocking(move || saves::set_icon(&id, png_base64))
        .await
        .map_err(|e| AppError::new("IMAGE", e.to_string()))??;
    Ok(view(&state, meta))
}

#[tauri::command]
pub async fn save_duplicate(state: S<'_>, id: String) -> AppResult<SaveView> {
    ensure_idle(&state, &id, "Duplicating")?;
    let meta = tauri::async_runtime::spawn_blocking(move || saves::duplicate(&id, None))
        .await
        .map_err(|e| AppError::new("SAVE", e.to_string()))??;
    Ok(view(&state, meta))
}

#[tauri::command]
pub async fn save_delete(state: S<'_>, id: String) -> AppResult<()> {
    ensure_idle(&state, &id, "Deleting")?;
    state.runs.lock().unwrap().remove(&id);
    tauri::async_runtime::spawn_blocking(move || saves::delete(&id)).await.map_err(|e| AppError::new("SAVE", e.to_string()))?
}

#[tauri::command]
pub async fn save_summary(id: String) -> AppResult<saves::SaveSummary> {
    let meta = saves::load(&id)?;
    tauri::async_runtime::spawn_blocking(move || saves::summary(&meta)).await.map_err(|e| AppError::new("SAVE", e.to_string()))
}

#[tauri::command]
pub fn save_open_folder(app: AppHandle, id: String, sub: Option<String>) -> AppResult<()> {
    let meta = saves::load(&id)?;
    let dir = match sub.as_deref() {
        None | Some("game") => meta.game_dir(),
        Some(s) if saves::GAME_SUBDIRS.contains(&s) => meta.game_dir().join(s),
        Some(_) => bail!("FOLDER", "Unknown folder"),
    };
    std::fs::create_dir_all(&dir)?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| AppError::new("FOLDER", e.to_string()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettingsView {
    client: Value,
    onboarding: Value,
    memory_mb: Option<u32>,
    default_memory_mb: u32,
    runtime: Option<Value>,
    locked: bool,
}

#[tauri::command]
pub fn save_settings_get(state: S, id: String) -> AppResult<SaveSettingsView> {
    let meta = saves::load(&id)?;
    Ok(SaveSettingsView {
        client: client::read(&meta),
        onboarding: client::onboarding(&meta),
        memory_mb: meta.memory_mb,
        default_memory_mb: state.settings.lock().unwrap().save_defaults.memory_mb,
        runtime: client::runtime(&meta, state.running_pid(&id)),
        locked: state.is_busy(&id),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettingsPatch {
    client: Option<Value>,
    memory_mb: Option<Option<u32>>,
}

#[tauri::command]
pub fn save_settings_set(state: S, id: String, patch: SaveSettingsPatch) -> AppResult<SaveSettingsView> {
    ensure_idle(&state, &id, "Changing these settings from the launcher")?;
    let mut meta = saves::load(&id)?;
    if let Some(c) = &patch.client {
        client::update(&meta, c)?;
    }
    if let Some(m) = patch.memory_mb {
        meta.memory_mb = m.map(|x| x.clamp(1024, 32768));
        meta.save()?;
    }
    save_settings_get(state, id)
}

#[tauri::command]
pub fn save_reset_intro(state: S, id: String) -> AppResult<()> {
    ensure_idle(&state, &id, "Resetting the introduction")?;
    client::reset_onboarding(&saves::load(&id)?)
}

// ---------------- Mods ----------------

#[tauri::command]
pub async fn mods_list(id: String) -> AppResult<mods::ModList> {
    let meta = saves::load(&id)?;
    tauri::async_runtime::spawn_blocking(move || mods::list(&meta)).await.map_err(|e| AppError::new("MODS", e.to_string()))?
}

#[tauri::command]
pub async fn mods_add(state: S<'_>, id: String, paths: Vec<String>) -> AppResult<Vec<String>> {
    ensure_idle(&state, &id, "Changing mods")?;
    let meta = saves::load(&id)?;
    tauri::async_runtime::spawn_blocking(move || mods::add(&meta, &paths)).await.map_err(|e| AppError::new("MODS", e.to_string()))?
}

#[tauri::command]
pub fn mods_set_enabled(state: S, id: String, file: String, enabled: bool) -> AppResult<()> {
    ensure_idle(&state, &id, "Changing mods")?;
    mods::set_enabled(&saves::load(&id)?, &file, enabled)
}

#[tauri::command]
pub fn mods_remove(state: S, id: String, file: String) -> AppResult<()> {
    ensure_idle(&state, &id, "Changing mods")?;
    mods::remove(&saves::load(&id)?, &file)
}

#[tauri::command]
pub fn mods_restore_component(state: S, id: String, mod_id: String) -> AppResult<()> {
    ensure_idle(&state, &id, "Changing mods")?;
    managed::restore(&saves::load(&id)?, &mod_id)
}

// ---------------- Install & Launch ----------------

fn set_phase(app: &AppHandle, state: &AppState, id: &str, phase: Option<RunPhase>) {
    {
        let mut runs = state.runs.lock().unwrap();
        match phase {
            Some(p) => {
                runs.insert(id.to_string(), p);
            }
            None => {
                runs.remove(id);
            }
        }
    }
    emit_save(app, state, id);
}

async fn prepare_save(app: &AppHandle, state: &Arc<AppState>, meta: &SaveMeta) -> AppResult<install::Prepared> {
    let concurrency = state.settings.lock().unwrap().advanced.concurrent_downloads as usize;
    let app2 = app.clone();
    let state2 = state.clone();
    let id = meta.id.clone();
    let emit = move |p: install::Progress| {
        state2.runs.lock().unwrap().insert(id.clone(), RunPhase::Downloading { progress: Some(p) });
        emit_save(&app2, &state2, &id);
    };
    let prepared = install::prepare(meta, concurrency, &emit).await?;
    let mut updated = saves::load(&meta.id)?;
    updated.installed_stack = stack::for_version(&meta.minecraft_version).map(|s| s.key());
    updated.save()?;
    Ok(prepared)
}

/// Install or repair a Save's files without launching (no sign-in needed: these are public game files).
#[tauri::command]
pub async fn save_install(app: AppHandle, state: S<'_>, id: String) -> AppResult<()> {
    ensure_idle(&state, &id, "Installing")?;
    let meta = saves::load(&id)?;
    let state = state.inner().clone();
    set_phase(&app, &state, &id, Some(RunPhase::Downloading { progress: None }));
    tauri::async_runtime::spawn(async move {
        match prepare_save(&app, &state, &meta).await {
            Ok(_) => set_phase(&app, &state, &meta.id, None),
            Err(e) => set_phase(&app, &state, &meta.id, Some(RunPhase::Error { message: e.to_string() })),
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn save_launch(app: AppHandle, state: S<'_>, id: String) -> AppResult<()> {
    ensure_idle(&state, &id, "Launching again")?;
    let meta = saves::load(&id)?;
    if !stack::is_supported(&meta.minecraft_version) {
        bail!("UNSUPPORTED", "Minecraft {} is not supported by the Stabled client.", meta.minecraft_version);
    }
    if managed::bundled_client(&meta.minecraft_version).is_none() {
        bail!("CLIENT_MISSING", "This launcher build doesn't include the Stabled client for Minecraft {}.", meta.minecraft_version);
    }
    // Server-side enforcement: no valid Minecraft session with Java Edition → no process, no downloads.
    let (profile, token) = state.auth.launch_credentials().await?;
    let state = state.inner().clone();
    set_phase(&app, &state, &id, Some(RunPhase::Downloading { progress: None }));
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_game(&app, &state, &meta, profile, token).await {
            crate::log_warn!("Launch of {} failed: {}", meta.id, e);
            set_phase(&app, &state, &meta.id, Some(RunPhase::Error { message: e.to_string() }));
        }
    });
    Ok(())
}

async fn run_game(app: &AppHandle, state: &Arc<AppState>, meta: &SaveMeta, profile: crate::auth::Profile, token: String) -> AppResult<()> {
    let prepared = prepare_save(app, state, meta).await?;
    set_phase(app, state, &meta.id, Some(RunPhase::Launching));
    let settings = state.settings.lock().unwrap().clone();
    let memory = meta.memory_mb.unwrap_or(settings.save_defaults.memory_mb);
    let extra = settings::validate_jvm_args(&settings.advanced.jvm_args)?;
    let auth = launch::LaunchAuth {
        username: profile.name.clone(),
        uuid: profile.id.clone(),
        access_token: token.clone(),
        xuid: "0".into(),
        client_id: crate::config::get().microsoft_client_id.clone().unwrap_or_default(),
    };
    let (java, args) = launch::build(meta, &prepared, &auth, memory, &extra)?;
    let started = launch::spawn(meta, java, args, &token).await?;
    drop(token);
    let mut updated = saves::load(&meta.id)?;
    updated.last_played_at = Some(chrono::Utc::now().to_rfc3339());
    updated.launches += 1;
    updated.save()?;
    let started_at = chrono::Utc::now();
    set_phase(app, state, &meta.id, Some(RunPhase::Running { pid: started.pid, started_at: started_at.to_rfc3339() }));

    let window = app.get_webview_window("main");
    match settings.launcher.after_launch.as_str() {
        "minimize" => {
            if let Some(w) = &window {
                let _ = w.minimize();
            }
        }
        "hide" => {
            if let Some(w) = &window {
                let _ = w.hide();
            }
        }
        _ => {}
    }

    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    state.stoppers.lock().unwrap().insert(meta.id.clone(), stop_tx);
    let mut child = started.child;
    let status = tokio::select! {
        s = child.wait() => s.ok(),
        _ = stop_rx => {
            let _ = child.kill().await;
            None
        }
    };
    state.stoppers.lock().unwrap().remove(&meta.id);
    let ran_for = (chrono::Utc::now() - started_at).num_seconds();
    if settings.launcher.after_launch == "hide" {
        if let Some(w) = &window {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
    let code = status.and_then(|s| s.code());
    crate::log_info!("Save {} exited with {:?} after {}s", meta.id, code, ran_for);
    let crashed = matches!(code, Some(c) if c != 0);
    if crashed {
        set_phase(app, state, &meta.id, Some(RunPhase::Error {
            message: format!("Minecraft closed unexpectedly (exit code {}). Open the Save's logs folder for details.", code.unwrap_or(-1)),
        }));
    } else {
        set_phase(app, state, &meta.id, None);
    }
    let _ = app.emit("game-exited", json!({ "id": meta.id, "code": code, "seconds": ran_for }));
    Ok(())
}

#[tauri::command]
pub fn save_stop(state: S, id: String) -> AppResult<()> {
    match state.stoppers.lock().unwrap().remove(&id) {
        Some(tx) => {
            let _ = tx.send(());
            Ok(())
        }
        None => bail!("SAVE", "This Save isn't running."),
    }
}

#[tauri::command]
pub fn save_clear_error(app: AppHandle, state: S, id: String) {
    if matches!(state.phase(&id), Some(RunPhase::Error { .. })) {
        set_phase(&app, &state, &id, None);
    }
}

// ---------------- Account ----------------

#[tauri::command]
pub fn auth_status(state: S) -> AuthStatus {
    state.auth.status()
}

#[tauri::command]
pub async fn auth_restore(app: AppHandle, state: S<'_>) -> AppResult<AuthStatus> {
    let s = state.auth.restore().await;
    let _ = app.emit("auth-changed", &s);
    Ok(s)
}

#[tauri::command]
pub async fn auth_sign_in(app: AppHandle, state: S<'_>) -> AppResult<AuthStatus> {
    let opener = app.clone();
    let r = state
        .auth
        .sign_in(move |url| opener.opener().open_url(url, None::<&str>).map_err(|e| AppError::new("AUTH_FAILED", format!("Couldn't open your browser: {e}"))))
        .await;
    let _ = app.emit("auth-changed", state.auth.status());
    r
}

#[tauri::command]
pub fn auth_cancel(state: S) {
    state.auth.cancel();
}

#[tauri::command]
pub fn auth_sign_out(app: AppHandle, state: S) -> AuthStatus {
    let s = state.auth.sign_out();
    let _ = app.emit("auth-changed", &s);
    s
}

// ---------------- News ----------------

#[tauri::command]
pub async fn news_get(state: S<'_>, cache_only: Option<bool>) -> AppResult<news::NewsResult> {
    let show = state.settings.lock().unwrap().launcher.show_minecraft_news;
    let r = news::get(show, cache_only.unwrap_or(false)).await;
    *state.news.lock().unwrap() = r.items.clone();
    Ok(r)
}

#[tauri::command]
pub async fn news_detail(state: S<'_>, id: String) -> AppResult<String> {
    let item = state.news.lock().unwrap().iter().find(|i| i.id == id).cloned();
    match item {
        Some(i) => news::detail(&i).await,
        None => bail!("NEWS", "That article is no longer available"),
    }
}

// ---------------- External links & folders ----------------

#[tauri::command]
pub fn open_external(app: AppHandle, url: String) -> AppResult<()> {
    let parsed = url::Url::parse(&url).map_err(|_| AppError::new("LINK", "Invalid link"))?;
    if parsed.scheme() != "https" {
        bail!("LINK", "Only secure (https) links can be opened");
    }
    app.opener().open_url(parsed.as_str(), None::<&str>).map_err(|e| AppError::new("LINK", e.to_string()))
}

#[tauri::command]
pub fn open_discord_invite(app: AppHandle) -> AppResult<()> {
    match crate::config::get().discord_invite_url.clone() {
        Some(u) => open_external(app, u),
        None => bail!("UNCONFIGURED", "The Discord invite isn't configured in this build (DISCORD_INVITE_URL)."),
    }
}

#[tauri::command]
pub fn open_data_folder(app: AppHandle, kind: String) -> AppResult<()> {
    let dir = match kind.as_str() {
        "data" => paths::root().clone(),
        "logs" => paths::logs_dir(),
        "shared" => paths::shared().clone(),
        _ => bail!("FOLDER", "Unknown folder"),
    };
    std::fs::create_dir_all(&dir)?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| AppError::new("FOLDER", e.to_string()))
}

/// Deletes shared downloads (game files, libraries, assets, Java, mod files). Saves and their worlds are untouched;
/// files are downloaded again on the next launch.
#[tauri::command]
pub async fn shared_cache_clear(state: S<'_>) -> AppResult<u64> {
    if state.any_busy() {
        bail!("LOCKED", "Close running Saves and wait for installs to finish first.");
    }
    tauri::async_runtime::spawn_blocking(|| {
        let size = crate::util::fsx::dir_size(paths::shared());
        if paths::shared().exists() {
            std::fs::remove_dir_all(paths::shared())?;
        }
        std::fs::create_dir_all(paths::shared())?;
        Ok(size)
    })
    .await
    .map_err(|e| AppError::new("CACHE", e.to_string()))?
}

// ---------------- Optimiser ----------------

/// Every Java runtime Stabled has installed (Java 17, 21 and/or 25, depending on the Saves' Minecraft versions).
fn java_runtimes() -> Vec<std::path::PathBuf> {
    let mut components: Vec<String> = stack::all().iter().map(|s| s.java.component.clone()).collect();
    components.sort();
    components.dedup();
    components.iter().map(|c| crate::minecraft::java::executable(c)).filter(|p| p.is_file()).collect()
}

#[tauri::command]
pub fn optimiser_view() -> optimiser::OptimiserView {
    optimiser::view(&java_runtimes())
}

#[tauri::command]
pub async fn optimiser_apply(id: String) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || optimiser::apply(&id, &java_runtimes()))
        .await
        .map_err(|e| AppError::new("OPTIMISER", e.to_string()))?
}

#[tauri::command]
pub async fn optimiser_restore(id: String) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || optimiser::restore(&id, &java_runtimes()))
        .await
        .map_err(|e| AppError::new("OPTIMISER", e.to_string()))?
}
