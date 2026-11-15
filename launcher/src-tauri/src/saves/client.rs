//! Per-Save Stabled client settings (`stabled/client.json`) and introduction state (`stabled/onboarding.json`),
//! shared with the in-game client (schema: docs/CONTRACTS.md). The launcher edits them only while the Save is not
//! running; while it runs, the game owns them.

use super::SaveMeta;
use crate::bail;
use crate::error::AppResult;
use crate::settings::SaveDefaults;
use crate::util::fsx;
use serde_json::{json, Value};

pub fn write_defaults(meta: &SaveMeta, d: &SaveDefaults) -> AppResult<()> {
    let client = json!({
        "version": 1,
        "fps": { "enabled": d.fps_enabled, "position": "top-left", "scale": 1.0, "color": "theme", "background": "panel" },
        "motionBlur": { "enabled": d.motion_blur_enabled, "strength": 40 },
        "discord": { "enabled": d.discord_enabled, "showElapsed": true },
        "performance": { "preset": d.performance_preset },
        "menus": { "styled": true }
    });
    fsx::write_json(&meta.stabled_dir().join("client.json"), &client)?;
    fsx::write_json(&meta.stabled_dir().join("onboarding.json"), &json!({ "version": 1, "completed": false, "skipped": false, "step": 1 }))
}

pub fn read(meta: &SaveMeta) -> Value {
    let mut v = fsx::read_json_value(&meta.stabled_dir().join("client.json"));
    // Fill documented defaults for anything missing so the UI always has a complete view.
    let defaults = json!({
        "fps": { "enabled": true, "position": "top-left", "scale": 1.0, "color": "theme", "background": "panel" },
        "motionBlur": { "enabled": false, "strength": 40 },
        "discord": { "enabled": true, "showElapsed": true },
        "performance": { "preset": "balanced" },
        "menus": { "styled": true }
    });
    for (section, values) in defaults.as_object().unwrap() {
        if !v[section].is_object() {
            v[section] = json!({});
        }
        for (k, dv) in values.as_object().unwrap() {
            if v[section].get(k).is_none() {
                v[section][k] = dv.clone();
            }
        }
    }
    v
}

const ALLOWED: &[(&str, &str, &[&str])] = &[
    ("fps", "enabled", &[]),
    ("fps", "position", &["top-left", "top-right", "bottom-left", "bottom-right"]),
    ("fps", "scale", &[]),
    ("fps", "color", &["theme", "white", "green", "yellow", "cyan"]),
    ("fps", "background", &["panel", "shadow", "none"]),
    ("motionBlur", "enabled", &[]),
    ("motionBlur", "strength", &[]),
    ("discord", "enabled", &[]),
    ("discord", "showElapsed", &[]),
    ("performance", "preset", &["performance", "balanced", "quality", "custom"]),
    ("menus", "styled", &[]),
];

/// Applies a validated partial update `{ section: { key: value } }`.
pub fn update(meta: &SaveMeta, patch: &Value) -> AppResult<Value> {
    let mut v = read(meta);
    for (section, values) in patch.as_object().into_iter().flatten() {
        for (key, value) in values.as_object().into_iter().flatten() {
            let Some((_, _, choices)) = ALLOWED.iter().find(|(s, k, _)| s == section && k == key) else {
                bail!("SETTINGS", "Unknown setting {section}.{key}");
            };
            let ok = match value {
                Value::Bool(_) => choices.is_empty() && matches!(key.as_str(), "enabled" | "showElapsed" | "styled"),
                Value::String(s) => choices.contains(&s.as_str()),
                Value::Number(n) => match (section.as_str(), key.as_str()) {
                    ("fps", "scale") => [0.75, 1.0, 1.25, 1.5, 2.0].iter().any(|x| (n.as_f64().unwrap_or(0.0) - x).abs() < 1e-6),
                    ("motionBlur", "strength") => n.as_f64().map(|x| (10.0..=100.0).contains(&x)).unwrap_or(false),
                    _ => false,
                },
                _ => false,
            };
            if !ok {
                bail!("SETTINGS", "Invalid value for {section}.{key}");
            }
            v[section.as_str()][key.as_str()] = value.clone();
        }
    }
    fsx::write_json(&meta.stabled_dir().join("client.json"), &v)?;
    Ok(v)
}

pub fn onboarding(meta: &SaveMeta) -> Value {
    let v = fsx::read_json_value(&meta.stabled_dir().join("onboarding.json"));
    json!({
        "completed": v["completed"].as_bool().unwrap_or(false),
        "skipped": v["skipped"].as_bool().unwrap_or(false),
        "step": v["step"].as_u64().unwrap_or(1).clamp(1, 5),
        "completedAt": v["completedAt"].clone(),
    })
}

/// "Show the introduction again": takes effect on the Save's next launch.
pub fn reset_onboarding(meta: &SaveMeta) -> AppResult<()> {
    fsx::write_json(&meta.stabled_dir().join("onboarding.json"), &json!({ "version": 1, "completed": false, "skipped": false, "step": 1 }))
}

/// Live status written by the running game; ignored unless that game process is the one Stabled is tracking.
pub fn runtime(meta: &SaveMeta, running_pid: Option<u32>) -> Option<Value> {
    let v = fsx::read_json_value(&meta.stabled_dir().join("runtime.json"));
    let pid = v["pid"].as_u64()? as u32;
    (Some(pid) == running_pid).then_some(v)
}
