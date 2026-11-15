//! Mod compatibility check before launch: wrong Minecraft versions, mods for other loaders, duplicate mods, missing
//! requirements and declared incompatibilities (from each mod's metadata, see `mods.rs`), plus known conflicts
//! between popular mods that their metadata doesn't declare.

use crate::error::AppResult;
use crate::mods::{self, ModEntry};
use crate::saves::SaveMeta;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    /// "error" (the game will probably not start, or the mod won't load) or "warning".
    pub level: String,
    /// "minecraft-version" | "loader" | "duplicate" | "dependency" | "breaks" | "known-conflict" | "invalid" | "other"
    pub kind: String,
    /// Mod file the problem belongs to, if one.
    pub file: Option<String>,
    pub mod_name: Option<String>,
    pub message: String,
    /// What to do about it.
    pub fix: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub problems: Vec<Problem>,
    pub checked_mods: usize,
}

/// Known conflicts: (mod id, other mod id, explanation, fix). Both must be enabled.
const CONFLICTS: &[(&str, &str, &str, &str)] = &[
    ("optifabric", "sodium", "OptiFine (through OptiFabric) and Sodium both replace Minecraft's renderer and crash together.", "Keep Sodium (and Iris for shaders) and remove OptiFabric/OptiFine."),
    ("vulkanmod", "sodium", "VulkanMod and Sodium both replace Minecraft's renderer and can't run together.", "Keep one of them: disable VulkanMod or Sodium."),
    ("vulkanmod", "iris", "Iris needs Sodium's OpenGL renderer, which VulkanMod replaces.", "Disable VulkanMod to use Iris shaders."),
    ("canvas", "sodium", "Canvas and Sodium both replace Minecraft's renderer.", "Keep one renderer: disable Canvas or Sodium."),
    ("canvas", "iris", "Canvas and Iris both replace Minecraft's rendering pipeline.", "Disable Canvas to use Iris."),
    ("optifabric", "iris", "OptiFine (through OptiFabric) and Iris both load shader packs and conflict.", "Remove OptiFabric/OptiFine and use Iris for shaders."),
    ("starlight", "phosphor", "Starlight and Phosphor both rewrite the lighting engine.", "Keep one of them (Starlight is the newer one)."),
    ("rubidium", "sodium", "Rubidium is a Forge port of Sodium; it can't run alongside Sodium.", "Remove Rubidium."),
    ("embeddium", "sodium", "Embeddium is a Forge port of Sodium; it can't run alongside Sodium.", "Remove Embeddium."),
    ("oculus", "iris", "Oculus is a Forge port of Iris; it can't run alongside Iris.", "Remove Oculus."),
    ("betterfps", "sodium", "BetterFps changes the same rendering code as Sodium.", "Remove BetterFps; Sodium covers it."),
    ("exordium", "immediatelyfast", "Exordium and ImmediatelyFast both batch HUD rendering and can draw parts of the HUD incorrectly together.", "Disable one of them."),
];

/// Mods that are obsolete or harmful on modern versions, with the reason.
const OBSOLETE: &[(&str, &str)] = &[
    ("starlight", "Minecraft 1.20 includes Starlight's lighting improvements; the mod is discontinued and can cause lighting bugs."),
    ("phosphor", "Phosphor is discontinued; Minecraft's own lighting engine is faster now."),
    ("lazydfu", "Not needed since Minecraft 1.19.4, which loads DataFixerUpper lazily by itself."),
    ("smoothboot", "Discontinued; it can slow down start-up on modern versions."),
];

fn kind_of(message: &str) -> &'static str {
    if message.starts_with("Made for Minecraft") {
        "minecraft-version"
    } else if message.contains("Stabled Saves use Fabric") || message.contains("No Fabric mod metadata") {
        "loader"
    } else if message.starts_with("Requires") {
        "dependency"
    } else if message.contains("incompatible with") {
        "breaks"
    } else if message.contains("not a valid mod archive") {
        "invalid"
    } else if message.contains("files provide the mod") {
        "duplicate"
    } else {
        "other"
    }
}

fn fix_for(kind: &str, e: Option<&ModEntry>, minecraft: &str) -> Option<String> {
    let name = e.map(|e| e.name.clone()).unwrap_or_else(|| "the mod".into());
    Some(match kind {
        "minecraft-version" => format!("Download {name}'s build for Minecraft {minecraft} (the Store shows compatible versions), or disable it."),
        "loader" => format!("Get the Fabric version of {name}, or remove it."),
        "dependency" => "Install the required mod (the Store can find it) or disable this mod.".into(),
        "breaks" => format!("Disable {name} or the mod it conflicts with."),
        "invalid" => "Remove the file and download the mod again.".into(),
        "duplicate" => "Keep the newest file and disable or remove the others.".into(),
        _ => return None,
    })
}

pub fn check(meta: &SaveMeta) -> AppResult<Report> {
    let list = mods::list(meta)?;
    let mut problems = Vec::new();
    for issue in &list.issues {
        let kind = kind_of(&issue.message);
        problems.push(Problem {
            level: issue.level.clone(),
            kind: kind.into(),
            file: None,
            mod_name: None,
            message: issue.message.clone(),
            fix: fix_for(kind, None, &meta.minecraft_version),
        });
    }
    for e in list.mods.iter().filter(|m| m.enabled) {
        for issue in &e.issues {
            let kind = kind_of(&issue.message);
            problems.push(Problem {
                level: issue.level.clone(),
                kind: kind.into(),
                file: Some(e.file.clone()),
                mod_name: Some(e.name.clone()),
                message: issue.message.clone(),
                fix: fix_for(kind, Some(e), &meta.minecraft_version),
            });
        }
    }
    problems.extend(known_conflicts(&list.mods));
    Ok(Report { problems, checked_mods: list.mods.iter().filter(|m| m.enabled).count() })
}

/// Known conflicts and obsolete mods among the enabled mods (also shown in the Mods tab).
pub fn known_conflicts(mods: &[ModEntry]) -> Vec<Problem> {
    let enabled: Vec<&ModEntry> = mods.iter().filter(|m| m.enabled).collect();
    let find = |id: &str| enabled.iter().find(|m| m.mod_id.as_deref() == Some(id)).copied();
    let mut out = Vec::new();
    for (a, b, why, fix) in CONFLICTS {
        if let (Some(x), Some(y)) = (find(a), find(b)) {
            out.push(Problem {
                level: "error".into(),
                kind: "known-conflict".into(),
                file: Some(x.file.clone()),
                mod_name: Some(x.name.clone()),
                message: format!("{} and {} are known not to work together. {why}", x.name, y.name),
                fix: Some((*fix).into()),
            });
        }
    }
    for (id, why) in OBSOLETE {
        if let Some(x) = find(id) {
            out.push(Problem {
                level: "warning".into(),
                kind: "known-conflict".into(),
                file: Some(x.file.clone()),
                mod_name: Some(x.name.clone()),
                message: format!("{}: {why}", x.name),
                fix: Some(format!("Remove {}.", x.name)),
            });
        }
    }
    out
}
