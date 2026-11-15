//! PC Optimiser.
//!
//! Integration status: the Windows optimiser from the earlier project was not present in this repository, so none
//! of its actions are included or claimed. This module provides Stabled's own small set of conservative,
//! game-relevant Windows settings. Each action:
//! * detects its current state,
//! * changes nothing until the user explicitly applies it (never on startup or when a Save launches),
//! * records the previous value before changing it, and can restore it,
//! * asks for administrator rights only when it writes machine-wide settings.
//! New actions (including the original optimiser's, if its source is provided) plug in by adding to `ACTIONS`.

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

pub struct ActionInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub details: &'static str,
    pub requires_admin: bool,
    pub requires_restart: bool,
}

pub const ACTIONS: &[ActionInfo] = &[
    ActionInfo {
        id: "power-plan",
        title: "High performance power plan",
        description: "Keeps the CPU at higher clock speeds instead of saving power. Useful on desktops; uses more battery on laptops.",
        details: "Switches the active Windows power plan with powercfg. Restore switches back to the plan you had before.",
        requires_admin: false,
        requires_restart: false,
    },
    ActionInfo {
        id: "game-mode",
        title: "Windows Game Mode",
        description: "Lets Windows prioritise the game and pause some background activity (such as driver installs) while you play.",
        details: "Sets AutoGameModeEnabled under HKEY_CURRENT_USER\\Software\\Microsoft\\GameBar.",
        requires_admin: false,
        requires_restart: false,
    },
    ActionInfo {
        id: "background-recording",
        title: "Turn off background game recording",
        description: "Stops Xbox Game Bar from continuously recording gameplay in the background, which costs GPU and disk time.",
        details: "Sets GameDVR_Enabled (HKCU\\System\\GameConfigStore) and AppCaptureEnabled (HKCU\\…\\CurrentVersion\\GameDVR) to 0. You can still record manually after restoring.",
        requires_admin: false,
        requires_restart: false,
    },
    ActionInfo {
        id: "java-gpu",
        title: "Use the high-performance GPU for Minecraft",
        description: "On laptops with two GPUs, tells Windows to run Stabled's Java runtime on the faster graphics card.",
        details: "Adds a per-app graphics preference for Stabled's javaw.exe under HKCU\\Software\\Microsoft\\DirectX\\UserGpuPreferences (the same setting as Settings → System → Display → Graphics).",
        requires_admin: false,
        requires_restart: false,
    },
    ActionInfo {
        id: "gpu-scheduling",
        title: "Hardware-accelerated GPU scheduling",
        description: "Lets the graphics card manage its own memory scheduling, which can reduce latency on supported GPUs and drivers.",
        details: "Sets HwSchMode under HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers. Needs administrator permission and takes effect after restarting Windows.",
        requires_admin: true,
        requires_restart: true,
    },
];

fn history_path() -> std::path::PathBuf {
    crate::paths::optimiser_dir().join("history.json")
}

pub fn history() -> Value {
    fsx::read_json_value(&history_path())
}

#[cfg_attr(not(windows), allow(dead_code))]
fn write_history(h: &Value) -> AppResult<()> {
    fsx::write_json(&history_path(), h)
}

const INTEGRATION_MESSAGE: &str = "The Windows optimiser from the earlier project wasn't found in this repository, so its actions aren't included here. The settings below are Stabled's own; each is applied only when you choose Apply and can be restored.";

pub fn view(java_path: Option<std::path::PathBuf>) -> OptimiserView {
    let note = IntegrationNote { connected: false, message: INTEGRATION_MESSAGE.into() };
    #[cfg(windows)]
    {
        let h = history();
        let actions = ACTIONS
            .iter()
            .map(|a| {
                let detected = windows_impl::detect(a.id, java_path.as_deref());
                ActionView {
                    id: a.id.into(),
                    title: a.title.into(),
                    description: a.description.into(),
                    details: a.details.into(),
                    requires_admin: a.requires_admin,
                    requires_restart: a.requires_restart,
                    state: detected.state.into(),
                    current: detected.current,
                    target: detected.target,
                    can_restore: h.get(a.id).map(|e| e["previous"].is_object()).unwrap_or(false),
                    applied_at: h.get(a.id).and_then(|e| e["appliedAt"].as_str()).map(str::to_owned),
                    unavailable_reason: detected.reason,
                }
            })
            .collect();
        OptimiserView { supported: true, platform_message: None, original_optimiser: note, actions }
    }
    #[cfg(not(windows))]
    {
        let _ = java_path;
        OptimiserView {
            supported: false,
            platform_message: Some("The PC Optimiser changes Windows settings, so it is only available on Windows.".into()),
            original_optimiser: note,
            actions: ACTIONS
                .iter()
                .map(|a| ActionView {
                    id: a.id.into(),
                    title: a.title.into(),
                    description: a.description.into(),
                    details: a.details.into(),
                    requires_admin: a.requires_admin,
                    requires_restart: a.requires_restart,
                    state: "unavailable".into(),
                    current: "Windows only".into(),
                    target: String::new(),
                    can_restore: false,
                    applied_at: None,
                    unavailable_reason: Some("Only available on Windows.".into()),
                })
                .collect(),
        }
    }
}

/// Apply one action, recording what it replaced. Returns a short result message for the UI.
pub fn apply(id: &str, java_path: Option<std::path::PathBuf>) -> AppResult<String> {
    let Some(info) = ACTIONS.iter().find(|a| a.id == id) else { bail!("OPTIMISER", "Unknown action") };
    #[cfg(windows)]
    {
        let previous = windows_impl::run(id, "apply", java_path.as_deref(), None, info.requires_admin)?;
        let mut h = history();
        if !h.is_object() {
            h = json!({});
        }
        // Keep the *original* value if the action is applied again.
        let original = h.get(id).and_then(|e| e.get("previous").cloned()).filter(Value::is_object).unwrap_or(previous);
        h[id] = json!({ "appliedAt": chrono::Utc::now().to_rfc3339(), "previous": original });
        write_history(&h)?;
        crate::log_info!("Optimiser applied {id}");
        Ok(if info.requires_restart { format!("{} turned on. Restart Windows for it to take effect.", info.title) } else { format!("{} applied.", info.title) })
    }
    #[cfg(not(windows))]
    {
        let _ = (info, java_path, json!({}));
        bail!("OPTIMISER", "The PC Optimiser is only available on Windows.")
    }
}

pub fn restore(id: &str, java_path: Option<std::path::PathBuf>) -> AppResult<String> {
    let Some(info) = ACTIONS.iter().find(|a| a.id == id) else { bail!("OPTIMISER", "Unknown action") };
    let mut h = history();
    let Some(previous) = h.get(id).and_then(|e| e.get("previous").cloned()).filter(Value::is_object) else {
        bail!("OPTIMISER", "There is no recorded previous setting to restore.");
    };
    #[cfg(windows)]
    {
        windows_impl::run(id, "restore", java_path.as_deref(), Some(&previous), info.requires_admin)?;
        if let Some(o) = h.as_object_mut() {
            o.remove(id);
        }
        write_history(&h)?;
        crate::log_info!("Optimiser restored {id}");
        Ok(if info.requires_restart { format!("Previous setting restored. Restart Windows for it to take effect.") } else { "Previous setting restored.".into() })
    }
    #[cfg(not(windows))]
    {
        let _ = (info, java_path, previous, &mut h);
        bail!("OPTIMISER", "The PC Optimiser is only available on Windows.")
    }
}

/// Entry point for the elevated helper process (`stabled.exe --stabled-optimiser-elevated <id> <op> <file>`).
#[cfg(windows)]
pub fn run_elevated_helper(args: &[String]) -> i32 {
    windows_impl::elevated_main(args)
}
