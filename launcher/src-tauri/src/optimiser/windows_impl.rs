//! Windows implementation of the optimiser actions.

use crate::bail;
use crate::error::{AppError, AppResult};
use serde_json::{json, Value};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE};
use winreg::RegKey;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const HIGH_PERFORMANCE: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
const ULTIMATE: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";
const GAMEBAR: &str = r"Software\Microsoft\GameBar";
const GAME_CONFIG: &str = r"System\GameConfigStore";
const GAME_DVR: &str = r"Software\Microsoft\Windows\CurrentVersion\GameDVR";
const GPU_PREFS: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";
const GRAPHICS_DRIVERS: &str = r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers";
const LAYERS: &str = r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers";
const MOUSE: &str = r"Control Panel\Mouse";
const STICKY: &str = r"Control Panel\Accessibility\StickyKeys";
const FILTER: &str = r"Control Panel\Accessibility\Keyboard Response";
const TOGGLE: &str = r"Control Panel\Accessibility\ToggleKeys";
const BACKGROUND_APPS: &str = r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications";
const SEARCH: &str = r"Software\Microsoft\Windows\CurrentVersion\Search";
const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const VISUAL_FX: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects";
const MULTIMEDIA: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";
const FSO_FLAG: &str = "DISABLEDXMAXIMIZEDWINDOWEDMODE";

pub struct Detected {
    pub state: &'static str,
    pub current: String,
    pub target: String,
    pub reason: Option<String>,
}

fn d(state: &'static str, current: impl Into<String>, target: impl Into<String>, reason: Option<String>) -> Detected {
    Detected { state, current: current.into(), target: target.into(), reason }
}

fn hive(machine: bool) -> RegKey {
    RegKey::predef(if machine { HKEY_LOCAL_MACHINE } else { HKEY_CURRENT_USER })
}

fn read_dword(machine: bool, path: &str, name: &str) -> Option<u32> {
    hive(machine).open_subkey_with_flags(path, KEY_READ).ok()?.get_value::<u32, _>(name).ok()
}

fn read_string(machine: bool, path: &str, name: &str) -> Option<String> {
    hive(machine).open_subkey_with_flags(path, KEY_READ).ok()?.get_value::<String, _>(name).ok()
}

/// Snapshot of a registry value so it can be put back exactly (including "did not exist").
fn snapshot(machine: bool, path: &str, name: &str) -> Value {
    if let Some(v) = read_dword(machine, path, name) {
        json!({ "machine": machine, "path": path, "name": name, "exists": true, "kind": "dword", "value": v })
    } else if let Some(v) = read_string(machine, path, name) {
        json!({ "machine": machine, "path": path, "name": name, "exists": true, "kind": "string", "value": v })
    } else {
        json!({ "machine": machine, "path": path, "name": name, "exists": false })
    }
}

fn put_back(s: &Value) -> AppResult<()> {
    let machine = s["machine"].as_bool().unwrap_or(false);
    let path = s["path"].as_str().unwrap_or_default();
    let name = s["name"].as_str().unwrap_or_default();
    let allowed = [GAMEBAR, GAME_CONFIG, GAME_DVR, GPU_PREFS, GRAPHICS_DRIVERS, LAYERS, MOUSE, STICKY, FILTER, TOGGLE, BACKGROUND_APPS, SEARCH, PERSONALIZE, VISUAL_FX, MULTIMEDIA];
    if !allowed.contains(&path) {
        bail!("OPTIMISER", "Refusing to restore an unexpected registry location");
    }
    let (key, _) = hive(machine).create_subkey(path)?;
    if !s["exists"].as_bool().unwrap_or(false) {
        let _ = key.delete_value(name);
        return Ok(());
    }
    match s["kind"].as_str() {
        Some("dword") => key.set_value(name, &(s["value"].as_u64().unwrap_or(0) as u32))?,
        Some("string") => key.set_value(name, &s["value"].as_str().unwrap_or_default().to_string())?,
        _ => {}
    }
    Ok(())
}

fn set_dword(machine: bool, path: &str, name: &str, v: u32) -> AppResult<()> {
    let (key, _) = hive(machine).create_subkey(path)?;
    key.set_value(name, &v)?;
    Ok(())
}

fn set_string(machine: bool, path: &str, name: &str, v: &str) -> AppResult<()> {
    let (key, _) = hive(machine).create_subkey(path)?;
    key.set_value(name, &v.to_string())?;
    Ok(())
}

/// Accessibility "Flags" values are decimal strings; bit 0x4 is the keyboard shortcut (hotkey).
fn flags(path: &str, default: u32) -> u32 {
    read_string(false, path, "Flags").and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

const HOTKEY_BIT: u32 = 0x4;

/// Applies the mouse and accessibility values from the registry immediately, without signing out.
fn refresh_input_settings() {
    use windows::Win32::UI::Accessibility::{FILTERKEYS, STICKYKEYS, TOGGLEKEYS};
    use windows::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPIF_SENDCHANGE, SPI_GETFILTERKEYS, SPI_SETFILTERKEYS, SPI_SETMOUSE, SPI_SETSTICKYKEYS, SPI_SETTOGGLEKEYS,
    };
    let num = |name: &str| read_string(false, MOUSE, name).and_then(|v| v.trim().parse::<i32>().ok()).unwrap_or(0);
    let mut mouse = [num("MouseThreshold1"), num("MouseThreshold2"), num("MouseSpeed")];
    unsafe {
        let _ = SystemParametersInfoW(SPI_SETMOUSE, 0, Some(mouse.as_mut_ptr().cast()), SPIF_SENDCHANGE);
        let mut sk = STICKYKEYS { cbSize: std::mem::size_of::<STICKYKEYS>() as u32, dwFlags: windows::Win32::UI::Accessibility::STICKYKEYS_FLAGS(flags(STICKY, 510)) };
        let _ = SystemParametersInfoW(SPI_SETSTICKYKEYS, sk.cbSize, Some((&mut sk as *mut STICKYKEYS).cast()), SPIF_SENDCHANGE);
        let mut fk = FILTERKEYS { cbSize: std::mem::size_of::<FILTERKEYS>() as u32, ..Default::default() };
        if SystemParametersInfoW(SPI_GETFILTERKEYS, fk.cbSize, Some((&mut fk as *mut FILTERKEYS).cast()), Default::default()).is_ok() {
            fk.dwFlags = flags(FILTER, 126);
            let _ = SystemParametersInfoW(SPI_SETFILTERKEYS, fk.cbSize, Some((&mut fk as *mut FILTERKEYS).cast()), SPIF_SENDCHANGE);
        }
        let mut tk = TOGGLEKEYS { cbSize: std::mem::size_of::<TOGGLEKEYS>() as u32, dwFlags: flags(TOGGLE, 62) };
        let _ = SystemParametersInfoW(SPI_SETTOGGLEKEYS, tk.cbSize, Some((&mut tk as *mut TOGGLEKEYS).cast()), SPIF_SENDCHANGE);
    }
}

/// Above-normal priority and no Windows 11 power throttling ("efficiency mode") for the game process.
pub fn boost_process(child: &tokio::process::Child) {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Threading::{
        ProcessPowerThrottling, SetPriorityClass, SetProcessInformation, ABOVE_NORMAL_PRIORITY_CLASS, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
    };
    let Some(raw) = child.raw_handle() else { return };
    let handle = HANDLE(raw);
    unsafe {
        if let Err(e) = SetPriorityClass(handle, ABOVE_NORMAL_PRIORITY_CLASS) {
            crate::log_warn!("Couldn't raise the game's priority: {e}");
        }
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: 0,
        };
        let _ = SetProcessInformation(handle, ProcessPowerThrottling, (&state as *const PROCESS_POWER_THROTTLING_STATE).cast(), std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32);
    }
}

fn powercfg(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("powercfg").args(args).creation_flags(CREATE_NO_WINDOW).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn guid_re() -> regex::Regex {
    regex::Regex::new(r"(?i)([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})").unwrap()
}

fn active_scheme() -> Option<(String, String)> {
    let out = powercfg(&["/getactivescheme"])?;
    let guid = guid_re().captures(&out)?.get(1)?.as_str().to_ascii_lowercase();
    let name = out.split('(').nth(1).and_then(|s| s.split(')').next()).unwrap_or(&guid).trim().to_string();
    Some((guid, name))
}

fn schemes() -> Vec<String> {
    powercfg(&["/list"]).map(|o| guid_re().captures_iter(&o).map(|c| c[1].to_ascii_lowercase()).collect()).unwrap_or_default()
}

fn java_value_names(java: &[PathBuf]) -> Vec<String> {
    java.iter().filter(|p| p.is_file()).map(|p| p.to_string_lossy().into_owned()).collect()
}

pub fn detect(id: &str, java: &[PathBuf]) -> Detected {
    match id {
        "power-plan" => {
            let Some((guid, name)) = active_scheme() else {
                return d("unknown", "Couldn't read the power plan", "High performance", None);
            };
            if guid == HIGH_PERFORMANCE || guid == ULTIMATE {
                return d("applied", name, "High performance", None);
            }
            if !schemes().iter().any(|s| s == HIGH_PERFORMANCE) {
                return d("unavailable", name, "High performance", Some("This PC doesn't offer the High performance plan (common on laptops with Modern Standby). Use the power mode in Windows Settings → System → Power instead.".into()));
            }
            d("not_applied", name, "High performance", None)
        }
        "game-mode" => match read_dword(false, GAMEBAR, "AutoGameModeEnabled") {
            Some(1) => d("applied", "On", "On", None),
            Some(_) => d("not_applied", "Off", "On", None),
            None => d("applied", "On (Windows default)", "On", None),
        },
        "background-recording" => {
            let dvr = read_dword(false, GAME_CONFIG, "GameDVR_Enabled");
            let cap = read_dword(false, GAME_DVR, "AppCaptureEnabled");
            if dvr == Some(0) && cap == Some(0) {
                d("applied", "Off", "Off", None)
            } else {
                d("not_applied", if dvr == Some(1) || cap == Some(1) { "On" } else { "Windows default (may be on)" }, "Off", None)
            }
        }
        "java-gpu" => {
            let names = java_value_names(java);
            if names.is_empty() {
                return d("unavailable", "Java not installed yet", "High performance", Some("Install or launch a Save first so Stabled's Java runtime exists.".into()));
            }
            let prefs: Vec<Option<String>> = names.iter().map(|n| read_string(false, GPU_PREFS, n)).collect();
            let high = prefs.iter().filter(|v| v.as_deref().is_some_and(|v| v.contains("GpuPreference=2"))).count();
            if high == names.len() {
                d("applied", "High performance", "High performance", None)
            } else if high > 0 {
                d("not_applied", &format!("High performance for {high} of {} Java runtimes", names.len()), "High performance", None)
            } else if prefs.iter().any(|v| v.as_deref().is_some_and(|v| v.contains("GpuPreference=1"))) {
                d("not_applied", "Power saving", "High performance", None)
            } else {
                d("not_applied", "Let Windows decide", "High performance", None)
            }
        }
        "gpu-scheduling" => match read_dword(true, GRAPHICS_DRIVERS, "HwSchMode") {
            Some(2) => d("applied", "On", "On", None),
            Some(_) => d("not_applied", "Off", "On", None),
            None => d("unavailable", "Not reported by the graphics driver", "On", Some("Your graphics driver doesn't report support for hardware-accelerated GPU scheduling.".into())),
        },
        "fullscreen-optimizations" => {
            let names = java_value_names(java);
            if names.is_empty() {
                return d("unavailable", "Java not installed yet", "Off for Java", Some("Install or launch a Save first so Stabled's Java runtime exists.".into()));
            }
            let off = names.iter().filter(|n| read_string(false, LAYERS, n).is_some_and(|v| v.contains(FSO_FLAG))).count();
            if off == names.len() {
                d("applied", "Off for Java", "Off for Java", None)
            } else {
                d("not_applied", if off > 0 { "Off for some Java runtimes" } else { "On (Windows default)" }, "Off for Java", None)
            }
        }
        "mouse-acceleration" => match read_string(false, MOUSE, "MouseSpeed").as_deref().map(str::trim) {
            Some("0") => d("applied", "Off", "Off", None),
            _ => d("not_applied", "On (Enhance pointer precision)", "Off", None),
        },
        "sticky-keys" => {
            let on = [(STICKY, 510), (FILTER, 126), (TOGGLE, 62)].iter().filter(|(p, def)| flags(p, *def) & HOTKEY_BIT != 0).count();
            if on == 0 {
                d("applied", "Shortcuts off", "Shortcuts off", None)
            } else {
                d("not_applied", "Shift ×5 opens Sticky Keys", "Shortcuts off", None)
            }
        }
        "game-bar" => match read_dword(false, GAMEBAR, "UseNexusForGameBarEnabled") {
            Some(0) => d("applied", "Off", "Off", None),
            _ => d("not_applied", "On (Win+G opens it)", "Off", None),
        },
        "background-apps" => match read_dword(false, BACKGROUND_APPS, "GlobalUserDisabled") {
            Some(1) => d("applied", "Off", "Off", None),
            _ => d("not_applied", "Allowed", "Off", None),
        },
        "visual-effects" => {
            let transparency = read_dword(false, PERSONALIZE, "EnableTransparency");
            let fx = read_dword(false, VISUAL_FX, "VisualFXSetting");
            if transparency == Some(0) && fx == Some(2) {
                d("applied", "Best performance", "Best performance", None)
            } else {
                d("not_applied", if transparency == Some(0) { "Transparency off, animations on" } else { "Windows default (effects on)" }, "Best performance", None)
            }
        }
        "network-throttling" => {
            let idx = read_dword(true, MULTIMEDIA, "NetworkThrottlingIndex");
            let resp = read_dword(true, MULTIMEDIA, "SystemResponsiveness");
            if idx == Some(0xFFFF_FFFF) && resp.is_some_and(|r| r <= 10) {
                d("applied", "Not throttled", "Not throttled", None)
            } else {
                d("not_applied", &format!("Throttled (Windows default{})", resp.map(|r| format!(", {r}% reserved")).unwrap_or_default()), "Not throttled", None)
            }
        }
        _ => d("unknown", "", "", None),
    }
}

/// Performs `apply` or `restore`. Returns the snapshot of what `apply` replaced.
pub fn run(id: &str, op: &str, java: &[PathBuf], previous: Option<&Value>, requires_admin: bool) -> AppResult<Value> {
    if requires_admin && !is_elevated() {
        return run_elevated(id, op, previous);
    }
    perform(id, op, java, previous)
}

fn perform(id: &str, op: &str, java: &[PathBuf], previous: Option<&Value>) -> AppResult<Value> {
    if op == "restore" {
        let prev = previous.ok_or_else(|| AppError::new("OPTIMISER", "Nothing to restore"))?;
        if id == "power-plan" {
            let guid = prev["scheme"].as_str().unwrap_or_default().to_ascii_lowercase();
            if !schemes().iter().any(|s| *s == guid) {
                bail!("OPTIMISER", "Your previous power plan no longer exists, so it can't be restored.");
            }
            powercfg(&["/setactive", &guid]).ok_or_else(|| AppError::new("OPTIMISER", "Windows refused to change the power plan"))?;
        } else {
            for s in prev["values"].as_array().into_iter().flatten() {
                put_back(s)?;
            }
            if id == "mouse-acceleration" || id == "sticky-keys" {
                refresh_input_settings();
            }
        }
        return Ok(json!({}));
    }
    match id {
        "power-plan" => {
            let (current, _) = active_scheme().ok_or_else(|| AppError::new("OPTIMISER", "Couldn't read the current power plan"))?;
            if !schemes().iter().any(|s| s == HIGH_PERFORMANCE) {
                bail!("OPTIMISER", "The High performance plan isn't available on this PC.");
            }
            powercfg(&["/setactive", HIGH_PERFORMANCE]).ok_or_else(|| AppError::new("OPTIMISER", "Windows refused to change the power plan"))?;
            Ok(json!({ "scheme": current }))
        }
        "game-mode" => {
            let snap = vec![snapshot(false, GAMEBAR, "AutoGameModeEnabled")];
            set_dword(false, GAMEBAR, "AutoGameModeEnabled", 1)?;
            Ok(json!({ "values": snap }))
        }
        "background-recording" => {
            let snap = vec![snapshot(false, GAME_CONFIG, "GameDVR_Enabled"), snapshot(false, GAME_DVR, "AppCaptureEnabled")];
            set_dword(false, GAME_CONFIG, "GameDVR_Enabled", 0)?;
            set_dword(false, GAME_DVR, "AppCaptureEnabled", 0)?;
            Ok(json!({ "values": snap }))
        }
        "java-gpu" => {
            let names = java_value_names(java);
            if names.is_empty() {
                bail!("OPTIMISER", "Stabled's Java runtime isn't installed yet.");
            }
            let snap: Vec<Value> = names.iter().map(|n| snapshot(false, GPU_PREFS, n)).collect();
            let (key, _) = hive(false).create_subkey(GPU_PREFS)?;
            for name in &names {
                key.set_value(name, &"GpuPreference=2;".to_string())?;
            }
            Ok(json!({ "values": snap }))
        }
        "gpu-scheduling" => {
            if read_dword(true, GRAPHICS_DRIVERS, "HwSchMode").is_none() {
                bail!("OPTIMISER", "Your graphics driver doesn't report support for this setting.");
            }
            let snap = vec![snapshot(true, GRAPHICS_DRIVERS, "HwSchMode")];
            let key = hive(true).open_subkey_with_flags(GRAPHICS_DRIVERS, KEY_SET_VALUE)?;
            key.set_value("HwSchMode", &2u32)?;
            Ok(json!({ "values": snap }))
        }
        "fullscreen-optimizations" => {
            let names = java_value_names(java);
            if names.is_empty() {
                bail!("OPTIMISER", "Stabled's Java runtime isn't installed yet.");
            }
            let snap: Vec<Value> = names.iter().map(|n| snapshot(false, LAYERS, n)).collect();
            for n in &names {
                let current = read_string(false, LAYERS, n).unwrap_or_default();
                if current.contains(FSO_FLAG) {
                    continue;
                }
                let flags = current.trim().trim_start_matches('~').trim();
                let value = if flags.is_empty() { format!("~ {FSO_FLAG}") } else { format!("~ {flags} {FSO_FLAG}") };
                set_string(false, LAYERS, n, &value)?;
            }
            Ok(json!({ "values": snap }))
        }
        "mouse-acceleration" => {
            let snap = vec![snapshot(false, MOUSE, "MouseSpeed"), snapshot(false, MOUSE, "MouseThreshold1"), snapshot(false, MOUSE, "MouseThreshold2")];
            for name in ["MouseSpeed", "MouseThreshold1", "MouseThreshold2"] {
                set_string(false, MOUSE, name, "0")?;
            }
            refresh_input_settings();
            Ok(json!({ "values": snap }))
        }
        "sticky-keys" => {
            let snap = vec![snapshot(false, STICKY, "Flags"), snapshot(false, FILTER, "Flags"), snapshot(false, TOGGLE, "Flags")];
            for (path, default) in [(STICKY, 510), (FILTER, 126), (TOGGLE, 62)] {
                set_string(false, path, "Flags", &(flags(path, default) & !HOTKEY_BIT).to_string())?;
            }
            refresh_input_settings();
            Ok(json!({ "values": snap }))
        }
        "game-bar" => {
            let snap = vec![snapshot(false, GAMEBAR, "UseNexusForGameBarEnabled")];
            set_dword(false, GAMEBAR, "UseNexusForGameBarEnabled", 0)?;
            Ok(json!({ "values": snap }))
        }
        "background-apps" => {
            let snap = vec![snapshot(false, BACKGROUND_APPS, "GlobalUserDisabled"), snapshot(false, SEARCH, "BackgroundAppGlobalToggle")];
            set_dword(false, BACKGROUND_APPS, "GlobalUserDisabled", 1)?;
            set_dword(false, SEARCH, "BackgroundAppGlobalToggle", 0)?;
            Ok(json!({ "values": snap }))
        }
        "visual-effects" => {
            let snap = vec![snapshot(false, PERSONALIZE, "EnableTransparency"), snapshot(false, VISUAL_FX, "VisualFXSetting")];
            set_dword(false, PERSONALIZE, "EnableTransparency", 0)?;
            set_dword(false, VISUAL_FX, "VisualFXSetting", 2)?;
            Ok(json!({ "values": snap }))
        }
        "network-throttling" => {
            let snap = vec![snapshot(true, MULTIMEDIA, "NetworkThrottlingIndex"), snapshot(true, MULTIMEDIA, "SystemResponsiveness")];
            let key = hive(true).open_subkey_with_flags(MULTIMEDIA, KEY_SET_VALUE)?;
            key.set_value("NetworkThrottlingIndex", &0xFFFF_FFFFu32)?;
            key.set_value("SystemResponsiveness", &10u32)?;
            Ok(json!({ "values": snap }))
        }
        _ => bail!("OPTIMISER", "Unknown action"),
    }
}

fn is_elevated() -> bool {
    unsafe { windows::Win32::UI::Shell::IsUserAnAdmin().as_bool() }
}

/// Re-runs Stabled elevated for a single action and waits for it. The user sees the normal Windows UAC prompt.
fn run_elevated(id: &str, op: &str, previous: Option<&Value>) -> AppResult<Value> {
    let prev = previous.map(|p| json!({ id: p }));
    let mut results = run_many(&[id], op, prev.as_ref())?;
    results.remove(id).unwrap_or_else(|| Err("The elevated helper did not complete.".into())).map_err(|e| AppError::new("OPTIMISER", e))
}

/// Runs several machine-wide actions in one elevated helper (one UAC prompt). `previous` maps id → snapshot for
/// restores. Returns each action's snapshot (apply) or error message.
pub fn run_many(ids: &[&str], op: &str, previous: Option<&Value>) -> AppResult<std::collections::HashMap<String, Result<Value, String>>> {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let mut out = std::collections::HashMap::new();
    if is_elevated() {
        for id in ids {
            let prev = previous.and_then(|p| p.get(*id));
            out.insert(id.to_string(), perform(id, op, &[], prev).map_err(|e| e.to_string()));
        }
        return Ok(out);
    }
    let dir = std::env::temp_dir().join(format!("stabled-optimiser-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir)?;
    let request = dir.join("request.json");
    let result = dir.join("result.json");
    crate::util::fsx::write_json(&request, &json!({ "ids": ids, "op": op, "previous": previous.cloned().unwrap_or(Value::Null) }))?;
    let exe = std::env::current_exe()?;
    let params = format!("--stabled-optimiser-elevated \"{}\" \"{}\"", request.display(), result.display());
    let exe_w = HSTRING::from(exe.as_os_str());
    let params_w = HSTRING::from(params);
    let verb = HSTRING::from("runas");
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(exe_w.as_ptr()),
        lpParameters: PCWSTR(params_w.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    unsafe {
        if ShellExecuteExW(&mut info).is_err() {
            let _ = std::fs::remove_dir_all(&dir);
            bail!("ELEVATION_DECLINED", "Administrator permission was not granted, so nothing was changed.");
        }
        WaitForSingleObject(info.hProcess, INFINITE);
        let mut code = 0u32;
        let _ = GetExitCodeProcess(info.hProcess, &mut code);
        let _ = CloseHandle(info.hProcess);
    }
    let body = crate::util::fsx::read_json_value(&result);
    let _ = std::fs::remove_dir_all(&dir);
    for id in ids {
        let r = &body["results"][*id];
        out.insert(
            id.to_string(),
            if r["ok"].as_bool() == Some(true) { Ok(r["previous"].clone()) } else { Err(r["error"].as_str().unwrap_or("The elevated helper did not complete.").to_string()) },
        );
    }
    Ok(out)
}

/// Runs inside the elevated helper process. Only machine-wide actions are accepted here.
pub fn elevated_main(args: &[String]) -> i32 {
    let (Some(req_path), Some(res_path)) = (args.first(), args.get(1)) else { return 2 };
    let req = crate::util::fsx::read_json_value(Path::new(req_path));
    let op = req["op"].as_str().unwrap_or_default();
    let admin_ids: Vec<&str> = super::ACTIONS.iter().filter(|a| a.requires_admin).map(|a| a.id).collect();
    let mut results = serde_json::Map::new();
    for id in req["ids"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        let outcome = if !admin_ids.contains(&id) || !(op == "apply" || op == "restore") {
            Err(AppError::new("OPTIMISER", "Unsupported elevated request"))
        } else {
            let prev = req["previous"].get(id).filter(|v| v.is_object());
            perform(id, op, &[], prev)
        };
        results.insert(
            id.to_string(),
            match outcome {
                Ok(previous) => json!({ "ok": true, "previous": previous }),
                Err(e) => json!({ "ok": false, "error": e.to_string() }),
            },
        );
    }
    let _ = crate::util::fsx::write_json(Path::new(res_path), &json!({ "results": results }));
    0
}
