//! PC Optimiser.
//!
//! Integration status: the Windows optimiser from the earlier project was not present in this repository, so none
//! of its actions are included or claimed. This module provides Stabled's own game-relevant settings. Each action:
//! * detects its current state,
//! * changes nothing until the user applies it (one at a time, or with Apply all), never on startup or launch,
//! * records the previous value before changing it, and can restore it,
//! * asks for administrator rights only when it writes machine-wide settings (Apply all asks once).
//!
//! Two kinds of action exist: Windows settings (`windows_impl`), and launch tweaks that Stabled itself applies to
//! the game when it starts a Save (`launch_tweaks`), which are recorded here and work on every platform they apply to.

#[cfg(windows)]
mod windows_impl;

use crate::bail;
use crate::error::AppResult;
use crate::util::fsx;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionView {
    pub id: String,
    pub title: String,
    pub description: String,
    pub details: String,
    pub category: String,
    pub impact: String,
    pub requires_admin: bool,
    pub requires_restart: bool,
    /// "applied" | "not_applied" | "unavailable" | "unknown"
    pub state: String,
    pub current: String,
    pub target: String,
    pub can_restore: bool,
    pub applied_at: Option<String>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimiserView {
    pub supported: bool,
    pub platform_message: Option<String>,
    pub original_optimiser: IntegrationNote,
    pub actions: Vec<ActionView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationNote {
    pub connected: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    pub id: String,
    pub title: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Scope {
    /// A Windows setting.
    Windows,
    /// Applied by Stabled when it launches a Save, on any platform.
    Launch,
    /// Applied by Stabled when it launches a Save, on Windows only.
    LaunchWindows,
}

pub struct ActionInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub details: &'static str,
    /// "power" | "graphics" | "input" | "background" | "launch"
    pub category: &'static str,
    /// "high" | "medium" | "low": how much it usually matters for Minecraft.
    pub impact: &'static str,
    pub requires_admin: bool,
    pub requires_restart: bool,
    pub scope: Scope,
}

pub const ACTIONS: &[ActionInfo] = &[
    ActionInfo {
        id: "power-plan",
        title: "High performance power plan",
        description: "Keeps the CPU at higher clock speeds instead of saving power. Useful on desktops; uses more battery on laptops.",
        details: "Switches the active Windows power plan with powercfg. Restore switches back to the plan you had before.",
        category: "power",
        impact: "high",
        requires_admin: false,
        requires_restart: false,
        scope: Scope::Windows,
    },
    ActionInfo {
        id: "game-priority",
        title: "High priority for Minecraft",
        description: "Runs Minecraft at above-normal CPU priority and stops Windows 11 from moving it to power-saving cores, so background apps can't steal frames.",
        details: "Applied by Stabled to the game process each time it launches a Save (SetPriorityClass ABOVE_NORMAL and SetProcessInformation with power throttling off). No Windows setting changes; Restore stops it for future launches.",
        category: "launch",
        impact: "high",
        requires_admin: false,
        requires_restart: false,
        scope: Scope::LaunchWindows,
    },
    ActionInfo {
        id: "java-gc",
        title: "Tuned Java memory management",
        description: "Starts Minecraft with garbage-collector settings tuned for games, which reduces lag spikes and stutter when memory is cleaned up.",
        details: "Stabled adds tuned G1 garbage collector flags when it launches a Save (-XX:+UseG1GC -XX:MaxGCPauseMillis=37 -XX:G1NewSizePercent=23 -XX:G1ReservePercent=20 -XX:G1HeapRegionSize=16M -XX:+ParallelRefProcEnabled -XX:+DisableExplicitGC -XX:+PerfDisableSharedMem and related). Skipped when your extra Java arguments choose a garbage collector. Restore stops adding them.",
        category: "launch",
        impact: "high",
        requires_admin: false,
        requires_restart: false,
        scope: Scope::Launch,
    },
    ActionInfo {
        id: "java-gpu",
        title: "Use the high-performance GPU for Minecraft",
        description: "On laptops with two GPUs, tells Windows to run Stabled's Java runtimes on the faster graphics card.",
        details: "Adds a per-app graphics preference for each of Stabled's Java runtimes (javaw.exe for Java 17, 21 and 25, whichever your Saves use) under HKCU\\Software\\Microsoft\\DirectX\\UserGpuPreferences (the same setting as Settings → System → Display → Graphics).",
        category: "graphics",
        impact: "high",
        requires_admin: false,
        requires_restart: false,
        scope: Scope::Windows,
    },
    ActionInfo {
        id: "fullscreen-optimizations",
        title: "Turn off fullscreen optimisations for Java",
        description: "Stops Windows from running fullscreen Minecraft as a borderless window, which can add input delay and frame pacing problems.",
        details: "Adds the DISABLEDXMAXIMIZEDWINDOWEDMODE compatibility flag for each of Stabled's Java runtimes under HKCU\\Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers (the same as the Compatibility tab's \"Disable fullscreen optimizations\").",
        category: "graphics",
        impact: "medium",
        requires_admin: false,
        requires_restart: false,
        scope: Scope::Windows,
    },
    ActionInfo {
        id: "gpu-scheduling",
        title: "Hardware-accelerated GPU scheduling",
        description: "Lets the graphics card manage its own memory scheduling, which can reduce latency on supported GPUs and drivers.",
        details: "Sets HwSchMode under HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers. Needs administrator permission and takes effect after restarting Windows.",
        category: "graphics",
        impact: "medium",
        requires_admin: true,
        requires_restart: true,
        scope: Scope::Windows,
    },
    ActionInfo {
        id: "mouse-acceleration",
        title: "Turn off mouse acceleration",
        description: "Turns off \"Enhance pointer precision\", so the same hand movement always turns your view by the same amount. Important for aiming and PvP.",
        details: "Sets MouseSpeed, MouseThreshold1 and MouseThreshold2 to 0 under HKCU\\Control Panel\\Mouse and applies it immediately (SystemParametersInfo SPI_SETMOUSE).",
        category: "input",
        impact: "high",
        requires_admin: false,
        requires_restart: false,
        scope: Scope::Windows,
    },
    ActionInfo {
        id: "sticky-keys",
        title: "Turn off the Sticky Keys shortcut",
        description: "Stops the Sticky Keys, Filter Keys and Toggle Keys pop-ups from appearing when you press Shift five times or hold it while sneaking.",
        details: "Clears the keyboard-shortcut bit in the Flags values under HKCU\\Control Panel\\Accessibility\\StickyKeys, Keyboard Response and ToggleKeys, and applies it immediately. The accessibility features themselves stay available in Windows Settings.",
        category: "input",
        impact: "medium",
        requires_admin: false,
        requires_restart: false,
        scope: Scope::Windows,
    },
    ActionInfo {
        id: "game-mode",
        title: "Windows Game Mode",
        description: "Lets Windows prioritise the game and pause some background activity (such as driver installs) while you play.",
        details: "Sets AutoGameModeEnabled under HKEY_CURRENT_USER\\Software\\Microsoft\\GameBar.",
        category: "background",
        impact: "medium",
        requires_admin: false,
        requires_restart: false,
        scope: Scope::Windows,
    },
    ActionInfo {
        id: "background-recording",
        title: "Turn off background game recording",
        description: "Stops Xbox Game Bar from continuously recording gameplay in the background, which costs GPU and disk time.",
        details: "Sets GameDVR_Enabled (HKCU\\System\\GameConfigStore) and AppCaptureEnabled (HKCU\\…\\CurrentVersion\\GameDVR) to 0. You can still record manually after restoring.",
        category: "background",
        impact: "high",
        requires_admin: false,
        requires_restart: false,
        scope: Scope::Windows,
    },
    ActionInfo {
        id: "game-bar",
        title: "Turn off the Game Bar overlay shortcut",
        description: "Stops Xbox Game Bar from opening over the game when you press Win+G or a controller's Xbox button.",
        details: "Sets UseNexusForGameBarEnabled to 0 under HKCU\\Software\\Microsoft\\GameBar.",
        category: "background",
        impact: "low",
        requires_admin: false,
        requires_restart: false,
        scope: Scope::Windows,
    },
    ActionInfo {
        id: "background-apps",
        title: "Stop apps running in the background",
        description: "Stops Microsoft Store apps from running in the background when you aren't using them, freeing memory and CPU time.",
        details: "Sets GlobalUserDisabled to 1 under HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\BackgroundAccessApplications and BackgroundAppGlobalToggle to 0 under …\\CurrentVersion\\Search. Desktop programs aren't affected.",
        category: "background",
        impact: "low",
        requires_admin: false,
        requires_restart: false,
        scope: Scope::Windows,
    },
    ActionInfo {
        id: "visual-effects",
        title: "Lighter Windows visual effects",
        description: "Turns off transparency and animation effects in Windows, which frees a little GPU time on lower-end PCs.",
        details: "Sets EnableTransparency to 0 (HKCU\\…\\Themes\\Personalize) and VisualFXSetting to 2, \"Adjust for best performance\" (HKCU\\…\\Explorer\\VisualEffects). Some effects change after you sign out and back in.",
        category: "background",
        impact: "low",
        requires_admin: false,
        requires_restart: false,
        scope: Scope::Windows,
    },
    ActionInfo {
        id: "network-throttling",
        title: "Network and multimedia responsiveness",
        description: "Stops Windows from throttling network traffic while media plays, and reserves less CPU for background multimedia. Can reduce lag spikes in multiplayer while music or videos play.",
        details: "Sets NetworkThrottlingIndex to 0xFFFFFFFF and SystemResponsiveness to 10 under HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile. Needs administrator permission and a restart.",
        category: "background",
        impact: "low",
        requires_admin: true,
        requires_restart: true,
        scope: Scope::Windows,
    },
];

fn history_path() -> std::path::PathBuf {
    crate::paths::optimiser_dir().join("history.json")
}

pub fn history() -> Value {
    fsx::read_json_value(&history_path())
}

fn write_history(h: &Value) -> AppResult<()> {
    fsx::write_json(&history_path(), h)
}

fn record(id: &str, previous: Value) -> AppResult<()> {
    let mut h = history();
    if !h.is_object() {
        h = json!({});
    }
    // Keep the *original* values if the action is applied again; record values it touches for the first time
    // (e.g. a Java runtime installed since the last Apply).
    let original = match h.get(id).and_then(|e| e.get("previous").cloned()).filter(Value::is_object) {
        Some(mut original) => {
            merge_new_values(&mut original, &previous);
            original
        }
        None => previous,
    };
    h[id] = json!({ "appliedAt": chrono::Utc::now().to_rfc3339(), "previous": original });
    write_history(&h)
}

fn forget(id: &str) -> AppResult<()> {
    let mut h = history();
    if let Some(o) = h.as_object_mut() {
        o.remove(id);
    }
    write_history(&h)
}

const INTEGRATION_MESSAGE: &str = "The Windows optimiser from the earlier project wasn't found in this repository, so its actions aren't included here. The settings below are Stabled's own; each is applied only when you choose Apply and can be restored.";

fn available_here(a: &ActionInfo) -> bool {
    cfg!(windows) || a.scope == Scope::Launch
}

fn action_view(a: &ActionInfo, h: &Value, java: &[std::path::PathBuf]) -> ActionView {
    let applied_at = h.get(a.id).and_then(|e| e["appliedAt"].as_str()).map(str::to_owned);
    let can_restore = h.get(a.id).map(|e| e["previous"].is_object()).unwrap_or(false);
    let (state, current, target, reason): (String, String, String, Option<String>) = if !available_here(a) {
        ("unavailable".into(), "Windows only".into(), String::new(), Some("Only available on Windows.".into()))
    } else if a.scope != Scope::Windows {
        if applied_at.is_some() {
            ("applied".into(), "On for every launch".into(), "On".into(), None)
        } else {
            ("not_applied".into(), "Off".into(), "On".into(), None)
        }
    } else {
        #[cfg(windows)]
        {
            let d = windows_impl::detect(a.id, java);
            (d.state.into(), d.current, d.target, d.reason)
        }
        #[cfg(not(windows))]
        {
            let _ = java;
            unreachable!()
        }
    };
    ActionView {
        id: a.id.into(),
        title: a.title.into(),
        description: a.description.into(),
        details: a.details.into(),
        category: a.category.into(),
        impact: a.impact.into(),
        requires_admin: a.requires_admin,
        requires_restart: a.requires_restart,
        state,
        current,
        target,
        can_restore,
        applied_at,
        unavailable_reason: reason,
    }
}

pub fn view(java: &[std::path::PathBuf]) -> OptimiserView {
    let note = IntegrationNote { connected: false, message: INTEGRATION_MESSAGE.into() };
    let h = history();
    let actions = ACTIONS.iter().map(|a| action_view(a, &h, java)).collect();
    OptimiserView {
        supported: true,
        platform_message: (!cfg!(windows)).then(|| "Windows settings can only be changed on Windows. The launch tweaks below work on this computer too.".to_string()),
        original_optimiser: note,
        actions,
    }
}

fn info(id: &str) -> AppResult<&'static ActionInfo> {
    match ACTIONS.iter().find(|a| a.id == id) {
        Some(a) => Ok(a),
        None => bail!("OPTIMISER", "Unknown action"),
    }
}

fn applied_message(a: &ActionInfo) -> String {
    if a.requires_restart {
        format!("{} turned on. Restart Windows for it to take effect.", a.title)
    } else if a.scope != Scope::Windows {
        format!("{} turned on. It applies from the next launch.", a.title)
    } else {
        format!("{} applied.", a.title)
    }
}

/// Apply one action, recording what it replaced. Returns a short result message for the UI.
pub fn apply(id: &str, java: &[std::path::PathBuf]) -> AppResult<String> {
    let a = info(id)?;
    if !available_here(a) {
        bail!("OPTIMISER", "This setting is only available on Windows.");
    }
    if a.scope != Scope::Windows {
        record(id, json!({ "stabled": true }))?;
    } else {
        #[cfg(windows)]
        {
            let previous = windows_impl::run(id, "apply", java, None, a.requires_admin)?;
            record(id, previous)?;
        }
        #[cfg(not(windows))]
        {
            let _ = java;
        }
    }
    crate::log_info!("Optimiser applied {id}");
    Ok(applied_message(a))
}

pub fn restore(id: &str, java: &[std::path::PathBuf]) -> AppResult<String> {
    let a = info(id)?;
    let h = history();
    let Some(previous) = h.get(id).and_then(|e| e.get("previous").cloned()).filter(Value::is_object) else {
        bail!("OPTIMISER", "There is no recorded previous setting to restore.");
    };
    if a.scope == Scope::Windows {
        #[cfg(windows)]
        windows_impl::run(id, "restore", java, Some(&previous), a.requires_admin)?;
        #[cfg(not(windows))]
        {
            let _ = (java, previous);
            bail!("OPTIMISER", "This setting is only available on Windows.");
        }
    }
    forget(id)?;
    crate::log_info!("Optimiser restored {id}");
    Ok(if a.requires_restart { "Previous setting restored. Restart Windows for it to take effect.".into() } else { "Previous setting restored.".into() })
}

/// Apply every available action that isn't applied yet. Machine-wide actions are batched into one administrator
/// prompt.
pub fn apply_all(java: &[std::path::PathBuf]) -> Vec<BatchResult> {
    let h = history();
    let pending: Vec<&ActionInfo> = ACTIONS.iter().filter(|a| available_here(a) && action_view(a, &h, java).state == "not_applied").collect();
    let mut out = Vec::new();
    let mut admin: Vec<&ActionInfo> = Vec::new();
    for a in pending {
        if a.requires_admin {
            admin.push(a);
            continue;
        }
        out.push(match apply(a.id, java) {
            Ok(m) => BatchResult { id: a.id.into(), title: a.title.into(), ok: true, message: m },
            Err(e) => BatchResult { id: a.id.into(), title: a.title.into(), ok: false, message: e.to_string() },
        });
    }
    if !admin.is_empty() {
        #[cfg(windows)]
        {
            let ids: Vec<&str> = admin.iter().map(|a| a.id).collect();
            match windows_impl::run_many(&ids, "apply", None) {
                Ok(results) => {
                    for a in &admin {
                        let r = results.get(a.id).cloned().unwrap_or(Err("The administrator helper didn't report a result.".into()));
                        out.push(match r.and_then(|prev| record(a.id, prev).map_err(|e| e.to_string())) {
                            Ok(()) => BatchResult { id: a.id.into(), title: a.title.into(), ok: true, message: applied_message(a) },
                            Err(e) => BatchResult { id: a.id.into(), title: a.title.into(), ok: false, message: e },
                        });
                    }
                }
                Err(e) => {
                    for a in &admin {
                        out.push(BatchResult { id: a.id.into(), title: a.title.into(), ok: false, message: e.to_string() });
                    }
                }
            }
        }
    }
    out
}

/// Restore every action Stabled applied. Machine-wide ones share one administrator prompt.
pub fn restore_all(java: &[std::path::PathBuf]) -> Vec<BatchResult> {
    let h = history();
    let mut out = Vec::new();
    let mut admin: Vec<(&ActionInfo, Value)> = Vec::new();
    for a in ACTIONS {
        let Some(prev) = h.get(a.id).and_then(|e| e.get("previous").cloned()).filter(Value::is_object) else { continue };
        if a.requires_admin && a.scope == Scope::Windows {
            admin.push((a, prev));
            continue;
        }
        out.push(match restore(a.id, java) {
            Ok(m) => BatchResult { id: a.id.into(), title: a.title.into(), ok: true, message: m },
            Err(e) => BatchResult { id: a.id.into(), title: a.title.into(), ok: false, message: e.to_string() },
        });
    }
    if !admin.is_empty() {
        #[cfg(windows)]
        {
            let ids: Vec<&str> = admin.iter().map(|(a, _)| a.id).collect();
            let previous: serde_json::Map<String, Value> = admin.iter().map(|(a, p)| (a.id.to_string(), p.clone())).collect();
            match windows_impl::run_many(&ids, "restore", Some(&Value::Object(previous))) {
                Ok(results) => {
                    for (a, _) in &admin {
                        let ok = matches!(results.get(a.id), Some(Ok(_)));
                        if ok {
                            let _ = forget(a.id);
                        }
                        out.push(BatchResult {
                            id: a.id.into(),
                            title: a.title.into(),
                            ok,
                            message: match results.get(a.id) {
                                Some(Ok(_)) => "Previous setting restored. Restart Windows for it to take effect.".into(),
                                Some(Err(e)) => e.clone(),
                                None => "The administrator helper didn't report a result.".into(),
                            },
                        });
                    }
                }
                Err(e) => {
                    for (a, _) in &admin {
                        out.push(BatchResult { id: a.id.into(), title: a.title.into(), ok: false, message: e.to_string() });
                    }
                }
            }
        }
        #[cfg(not(windows))]
        let _ = admin;
    }
    out
}

/// Tweaks Stabled applies when it launches a Save.
pub struct LaunchTweaks {
    pub gc_flags: bool,
    #[cfg_attr(not(windows), allow(dead_code))]
    pub high_priority: bool,
}

pub fn launch_tweaks() -> LaunchTweaks {
    let h = history();
    let on = |id: &str| h.get(id).map(|e| e["previous"].is_object()).unwrap_or(false);
    LaunchTweaks { gc_flags: on("java-gc"), high_priority: cfg!(windows) && on("game-priority") }
}

/// Garbage-collector flags for "Tuned Java memory management" (G1, tuned for a game client's short pauses).
pub const GC_FLAGS: &[&str] = &[
    "-XX:+UseG1GC",
    "-XX:+ParallelRefProcEnabled",
    "-XX:MaxGCPauseMillis=37",
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+DisableExplicitGC",
    "-XX:G1NewSizePercent=23",
    "-XX:G1MaxNewSizePercent=50",
    "-XX:G1HeapRegionSize=16M",
    "-XX:G1ReservePercent=20",
    "-XX:G1MixedGCCountTarget=3",
    "-XX:InitiatingHeapOccupancyPercent=20",
    "-XX:G1MixedGCLiveThresholdPercent=90",
    "-XX:SurvivorRatio=32",
    "-XX:MaxTenuringThreshold=1",
    "-XX:+PerfDisableSharedMem",
];

/// Raises the game's priority and turns off Windows 11 power throttling for it. Errors are logged, never fatal.
#[cfg(windows)]
pub fn boost_process(child: &tokio::process::Child) {
    windows_impl::boost_process(child);
}

/// Adds registry snapshots from `newer` whose (hive, path, name) isn't in `original` yet.
fn merge_new_values(original: &mut Value, newer: &Value) {
    let key = |v: &Value| (v["machine"].as_bool(), v["path"].as_str().map(str::to_owned), v["name"].as_str().map(str::to_owned));
    let Some(new_values) = newer["values"].as_array() else { return };
    let Some(values) = original.get_mut("values").and_then(Value::as_array_mut) else { return };
    for v in new_values {
        if !values.iter().any(|o| key(o) == key(v)) {
            values.push(v.clone());
        }
    }
}

/// Entry point for the elevated helper process (`stabled.exe --stabled-optimiser-elevated <request> <result>`).
#[cfg(windows)]
pub fn run_elevated_helper(args: &[String]) -> i32 {
    windows_impl::elevated_main(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_are_well_formed() {
        let mut ids = std::collections::HashSet::new();
        for a in ACTIONS {
            assert!(ids.insert(a.id), "duplicate id {}", a.id);
            assert!(["power", "graphics", "input", "background", "launch"].contains(&a.category));
            assert!(["high", "medium", "low"].contains(&a.impact));
            // Launch tweaks never need administrator rights.
            assert!(a.scope == Scope::Windows || !a.requires_admin);
        }
        assert!(ACTIONS.len() >= 14);
    }
}
