//! Crash explanations: turns crash reports, JVM crash files and game logs into plain-language explanations with
//! suggested fixes and a confidence level.
//!
//! The rules match what Minecraft, Fabric Loader, Mixin and the JVM actually print. Each finding quotes the line it
//! is based on, so the player (or someone helping them) can check it. When nothing specific matches, the report
//! says so with low confidence instead of guessing.

use crate::saves::SaveMeta;
use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub explanation: String,
    pub fixes: Vec<String>,
    /// "high" | "medium" | "low"
    pub confidence: String,
    /// The line the finding is based on.
    pub evidence: Option<String>,
    /// Mod ids involved.
    pub mods: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    /// What was analysed, e.g. "crash-reports/crash-2026-11-15_13.02.11-client.txt" or "logs/latest.log".
    pub file: Option<String>,
    pub exit_code: Option<i32>,
    /// The error Minecraft or Java reported, e.g. "java.lang.NullPointerException: …".
    pub headline: Option<String>,
    pub findings: Vec<Finding>,
    /// Mods whose code appears in the error's stack trace (from Mixin handler names and Mixin configs).
    pub mods_in_trace: Vec<String>,
}

/// Context that makes explanations specific to the Save.
pub struct Context<'a> {
    pub minecraft: &'a str,
    pub memory_mb: u32,
    /// Installed mod ids → display names.
    pub mods: &'a HashMap<String, String>,
}

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("crash regex"))
}

macro_rules! rx {
    ($p:expr) => {{
        static CELL: OnceLock<Regex> = OnceLock::new();
        re(&CELL, $p)
    }};
}

fn line_of(text: &str, m: regex::Match) -> String {
    let start = text[..m.start()].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = text[m.end()..].find('\n').map(|i| m.end() + i).unwrap_or(text.len());
    let line = text[start..end].trim();
    if line.chars().count() > 300 {
        line.chars().take(300).collect::<String>() + "…"
    } else {
        line.to_string()
    }
}

fn finding(id: &str, title: impl Into<String>, explanation: impl Into<String>, fixes: Vec<String>, confidence: &str, evidence: Option<String>, mods: Vec<String>) -> Finding {
    Finding { id: id.into(), title: title.into(), explanation: explanation.into(), fixes, confidence: confidence.into(), evidence, mods }
}

fn mod_label(ctx: &Context, id: &str) -> String {
    match ctx.mods.get(id) {
        Some(name) if name != id => format!("{name} ({id})"),
        _ => id.to_string(),
    }
}

/// Mod ids named by Mixin in a stack trace: handler methods are named `handler$abc000$modid$method`, and configs
/// `modid.mixins.json`.
pub fn mods_in_trace(text: &str, ctx: &Context) -> Vec<String> {
    let mut ids = BTreeSet::new();
    for c in rx!(r"\$[a-z]{3}\d{3}\$([a-z][a-z0-9_]*)\$").captures_iter(text) {
        ids.insert(c[1].to_string());
    }
    for c in rx!(r"\b([a-z][a-z0-9_\-]*)\.mixins?\.json").captures_iter(text) {
        ids.insert(c[1].to_string());
    }
    for c in rx!(r"from mod ([a-z][a-z0-9_\-]*)").captures_iter(text) {
        ids.insert(c[1].to_string());
    }
    // Keep ids that are installed mods when we know them (config names don't always equal the mod id).
    ids.into_iter()
        .filter(|id| !matches!(id.as_str(), "minecraft" | "java" | "fabricloader" | "mixinextras"))
        .map(|id| {
            if ctx.mods.contains_key(&id) {
                return id;
            }
            // "sodium-extra" config vs "sodium_extra" id and similar.
            let alt = id.replace('-', "_");
            if ctx.mods.contains_key(&alt) {
                return alt;
            }
            id
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// The error the game reported: a crash report's description and exception, or the last exception in a log.
pub fn headline(text: &str) -> Option<String> {
    if let Some(c) = rx!(r"(?m)^Description: (.+)$").captures(text) {
        let desc = c[1].trim().to_string();
        let after = &text[c.get(0).unwrap().end()..];
        if let Some(e) = rx!(r"(?m)^\s*((?:[a-z][\w$]*\.)+[A-Z][\w$]*(?:Exception|Error|Throwable)[^\n]*)").captures(after) {
            return Some(format!("{desc}: {}", e[1].trim()));
        }
        return Some(desc);
    }
    if let Some(c) = rx!(r"(?m)^#\s+(EXCEPTION_\w+|SIGSEGV|SIGBUS|SIGILL)[^\n]*").captures(text) {
        return Some(format!("The Java runtime crashed ({})", &c[1]));
    }
    rx!(r"(?m)^\s*(?:Caused by: |Exception in thread \S+ )?((?:[a-z][\w$]*\.)+[A-Z][\w$]*(?:Exception|Error)\b[^\n]*)")
        .captures_iter(text)
        .last()
        .map(|c| {
            let l = c[1].trim().to_string();
            if l.chars().count() > 240 {
                l.chars().take(240).collect::<String>() + "…"
            } else {
                l
            }
        })
}

pub fn analyze(text: &str, exit_code: Option<i32>, ctx: &Context) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    let mc = ctx.minecraft;
    let gb = |mb: u32| format!("{:.0}", (mb as f64 / 1024.0).ceil());

    if let Some(m) = rx!(r"Manually triggered debug crash").find(text) {
        out.push(finding(
            "manual",
            "The game was crashed on purpose (F3 + C)",
            "Holding F3 and C for ten seconds makes Minecraft crash deliberately so developers can test crash handling. Nothing is broken.",
            vec!["Nothing to fix. Release F3 + C sooner if you only wanted to copy coordinates.".into()],
            "high",
            Some(line_of(text, m)),
            vec![],
        ));
        return out;
    }

    // Fabric Loader refused the mod set: its own explanation is the best one, so quote it.
    if let Some(m) = rx!(r"Incompatible mods? (?:set )?found|Some of your mods are incompatible|Mod resolution (?:failed|encountered an incompatible mod set)").find(text) {
        let mut steps = Vec::new();
        let mut ids = BTreeSet::new();
        let section = &text[m.start()..];
        for c in rx!(r"(?m)^\s*-\s+((?:Replace|Remove|Install|Mod|Add|Update)[^\n]+)").captures_iter(section).take(12) {
            let line = c[1].trim().to_string();
            for id in rx!(r"\(([a-z][a-z0-9_\-]*)\)").captures_iter(&line) {
                ids.insert(id[1].to_string());
            }
            if !steps.contains(&line) {
                steps.push(line);
            }
        }
        let mut fixes: Vec<String> = steps.iter().filter(|s| !s.starts_with("Mod ")).cloned().collect();
        fixes.push("Open this Save's Mods tab: Stabled marks the mods involved and why.".into());
        let explanation = if steps.iter().any(|s| s.starts_with("Mod ")) {
            format!("Fabric checked your mods before starting and found problems: {}", steps.iter().filter(|s| s.starts_with("Mod ")).cloned().collect::<Vec<_>>().join(" "))
        } else {
            "Fabric checked your mods before starting and found that some need a different version of Minecraft or of another mod, or need a mod that isn't installed.".to_string()
        };
        out.push(finding("fabric-incompatible", "Mods are missing or the wrong version", explanation, fixes, "high", Some(line_of(text, m)), ids.into_iter().collect()));
    }

    if let Some(m) = rx!(r"DuplicateModsFoundException|Found duplicate mods?|Duplicate mod(?:s)? (?:found|detected)|ModResolutionException: Duplicate").find(text) {
        out.push(finding(
            "duplicates",
            "The same mod is installed twice",
            "Two files in the mods folder contain the same mod (often two versions of it). Fabric refuses to start when that happens.",
            vec!["In the Mods tab, keep the newest file of the duplicated mod and remove or disable the other.".into()],
            "high",
            Some(line_of(text, m)),
            vec![],
        ));
    }

    if let Some(m) = rx!(r"Could not reserve enough space for (?:\d+\w* )?object heap|Invalid maximum heap size|Initial heap size set to a larger value").find(text) {
        out.push(finding(
            "heap-reserve",
            "Java couldn't get the memory this Save asks for",
            format!("This Save asks for {} GB of memory, but Windows couldn't give Java that much in one piece.", gb(ctx.memory_mb)),
            vec![
                "Lower this Save's memory in Save settings → Memory (4 GB suits most modded Saves).".into(),
                "Close memory-heavy programs (browsers, video editors) and try again.".into(),
            ],
            "high",
            Some(line_of(text, m)),
            vec![],
        ));
    } else if let Some(m) = rx!(r"java\.lang\.OutOfMemoryError(?::\s*([^\n]+))?").find(text) {
        let kind = rx!(r"OutOfMemoryError:\s*([^\n]+)").captures(&text[m.start()..]).map(|c| c[1].trim().to_string()).unwrap_or_default();
        let suggest = ((ctx.memory_mb / 1024) + 2).clamp(4, 12);
        let (explanation, mut fixes) = if kind.contains("Metaspace") || kind.contains("Direct buffer") {
            (
                format!("Minecraft ran out of one of Java's special memory areas ({kind}). This usually comes from a mod or a very large resource or shader pack."),
                vec!["Remove recently added resource packs, shader packs or mods and try again.".to_string()],
            )
        } else {
            (
                format!("Minecraft used all the memory this Save allows ({} GB) and couldn't continue.", gb(ctx.memory_mb)),
                vec![format!("Give this Save more memory in Save settings → Memory (try {suggest} GB if your PC has enough RAM).")],
            )
        };
        fixes.push("Lower render distance, or use fewer or smaller resource packs.".into());
        out.push(finding("out-of-memory", "Minecraft ran out of memory", explanation, fixes, "high", Some(line_of(text, m)), vec![]));
    }

    if let Some(m) = rx!(r"UnsupportedClassVersionError|compiled by a more recent version of the Java Runtime \(class file version (\d+)").find(text) {
        let java = rx!(r"class file version (\d+)").captures(&text[m.start()..]).and_then(|c| c[1].parse::<u32>().ok()).map(|v| v.saturating_sub(44));
        let class = rx!(r"(?:UnsupportedClassVersionError:\s*)([\w/$.]+)").captures(&text[m.start()..]).map(|c| c[1].replace('/', "."));
        let needs = java.map(|j| format!(" Java {j}")).unwrap_or_default();
        out.push(finding(
            "java-version",
            "A mod needs a newer Java than this Minecraft version uses",
            format!(
                "A mod was built for{needs}, which Minecraft {mc} doesn't use. That almost always means the mod was made for a newer Minecraft version.{}",
                class.map(|c| format!(" The failing code is {c}.")).unwrap_or_default()
            ),
            vec![format!("Replace the mod with its build for Minecraft {mc}, or remove it."), "If you don't know which mod it is, use Find the broken mod.".into()],
            "high",
            Some(line_of(text, m)),
            vec![],
        ));
    }

    if let Some(m) = rx!(r"MixinApplyError|InvalidMixinException|InvalidInjectionException|InjectionError|MixinTransformerError|Critical injection failure|Mixin transformation of [^\n]* failed|MixinPrepareError").find(text) {
        let section = &text[m.start().saturating_sub(400)..(m.end() + 4000).min(text.len())];
        let mut ids: Vec<String> = rx!(r"from mod ([a-z][a-z0-9_\-]*)").captures_iter(section).map(|c| c[1].to_string()).collect();
        if ids.is_empty() {
            ids = rx!(r"\b([a-z][a-z0-9_\-]*)\.mixins?\.json").captures_iter(section).map(|c| c[1].to_string()).collect();
        }
        ids.dedup();
        let names: Vec<String> = ids.iter().map(|i| mod_label(ctx, i)).collect();
        let (title, explanation, confidence) = if names.is_empty() {
            ("A mod failed to hook into the game".to_string(), "A mod tried to change part of Minecraft that doesn't exist in this version, or that another mod already changed.".to_string(), "medium")
        } else {
            (
                format!("{} failed to hook into the game", names.join(", ")),
                format!("{} tried to change part of Minecraft that doesn't exist in this version, or that another mod already changed.", names.join(" and ")),
                "high",
            )
        };
        let mut fixes = vec![];
        if let Some(n) = names.first() {
            fixes.push(format!("Update {n} to its newest build for Minecraft {mc}, or disable it."));
        } else {
            fixes.push(format!("Check that every mod is made for Minecraft {mc}."));
        }
        fixes.push("If it started after adding another mod, those two mods may conflict: disable the newer one.".into());
        out.push(finding("mixin", title, explanation, fixes, confidence, Some(line_of(text, m)), ids));
    }

    if let Some(c) = rx!(r"Could not execute entrypoint stage '(\w+)' due to errors, provided by '([a-z0-9_\-]+)'").captures(text) {
        let id = c[2].to_string();
        out.push(finding(
            "entrypoint",
            format!("{} crashed while starting", mod_label(ctx, &id)),
            format!("{} threw an error while Minecraft was loading mods.", mod_label(ctx, &id)),
            vec![format!("Update {} or disable it.", mod_label(ctx, &id)), "If it needs another mod, the Mods tab says which.".into()],
            "high",
            Some(line_of(text, c.get(0).unwrap())),
            vec![id],
        ));
    }

    if let Some(m) = rx!(r"(?i)Sodium has detected an (?:incompatible|unsupported) graphics driver|Unsupported graphics driver|graphics driver is (?:too old|unsupported)").find(text) {
        out.push(finding(
            "driver-outdated",
            "Your graphics driver is too old",
            "A rendering mod (usually Sodium) detected a graphics driver with known bugs that crash Minecraft.",
            vec!["Update your graphics driver from NVIDIA, AMD or Intel's website (not only through Windows Update).".into()],
            "high",
            Some(line_of(text, m)),
            vec![],
        ));
    }

    if let Some(m) = rx!(r"Pixel format not accelerated|GLFW error 6554[23]|WGL: The driver does not appear to support OpenGL|Failed to create (?:the )?OpenGL context|No OpenGL context|OpenGL (?:3\.2|4\.\d) (?:core )?(?:profile )?is not supported|Couldn't set pixel format|Unable to initialize OpenGL").find(text) {
        out.push(finding(
            "opengl",
            "Your graphics driver can't start Minecraft",
            "Minecraft couldn't open its window with OpenGL. That happens with missing or generic graphics drivers, on Remote Desktop and in virtual machines.",
            vec![
                "Install the latest graphics driver from NVIDIA, AMD or Intel.".into(),
                "On laptops, run Minecraft on the faster GPU (PC Optimiser → Use the high-performance GPU).".into(),
                "Play on the PC itself rather than through Remote Desktop.".into(),
            ],
            "high",
            Some(line_of(text, m)),
            vec![],
        ));
    }

    let driver_dll = rx!(r"(?i)\b(ig\w*icd(?:32|64)\.dll|ig9icd\w*\.dll|atio6axx\.dll|atig6pxx\.dll|atioglxx\.dll|amdxc64\.dll|nvoglv(?:32|64)\.dll|nvd3dumx\.dll|libnvidia-glcore\.so[\w.]*|radeonsi_dri\.so|iris_dri\.so)");
    if let Some(m) = rx!(r"EXCEPTION_ACCESS_VIOLATION|SIGSEGV").find(text) {
        if let Some(dll) = driver_dll.find(text) {
            let vendor = match dll.as_str().to_ascii_lowercase() {
                d if d.starts_with("ig") || d.contains("iris") => "Intel",
                d if d.starts_with("ati") || d.starts_with("amd") || d.contains("radeon") => "AMD",
                _ => "NVIDIA",
            };
            out.push(finding(
                "driver-crash",
                format!("The {vendor} graphics driver crashed"),
                format!("Minecraft crashed inside the {vendor} graphics driver ({}). This is a driver problem, often triggered by shaders or overlays.", dll.as_str()),
                vec![
                    format!("Update or cleanly reinstall the {vendor} graphics driver."),
                    "Turn off overlays (Discord, GeForce Experience, MSI Afterburner/RivaTuner) and try again.".into(),
                    "If you use a shader pack, try without it.".into(),
                ],
                "high",
                Some(line_of(text, dll)),
                vec![],
            ));
        } else {
            out.push(finding(
                "native-crash",
                "Minecraft crashed outside Java",
                "The game crashed in native code: a graphics driver, an overlay, or a mod that uses native libraries.",
                vec![
                    "Update your graphics driver.".into(),
                    "Turn off overlays and screen recorders, then try again.".into(),
                    "If it keeps happening, use Find the broken mod.".into(),
                ],
                "medium",
                Some(line_of(text, m)),
                vec![],
            ));
        }
    }

    if let Some(m) = rx!(r"UnsatisfiedLinkError|Failed to locate library: \S*lwjgl|Failed to load library: \S*lwjgl").find(text) {
        out.push(finding(
            "natives",
            "A game library couldn't be loaded",
            "One of Minecraft's native libraries (LWJGL) is missing, damaged or blocked.",
            vec!["Use Repair on this Save: it checks every game file and replaces damaged ones. Your worlds aren't touched.".into(), "If your antivirus quarantined a file, allow Stabled's folder and repair again.".into()],
            "high",
            Some(line_of(text, m)),
            vec![],
        ));
    }

    if let Some(m) = rx!(r"ZipException|invalid LOC header|zip END header not found|Corrupt(?:ed)? (?:jar|zip)|error in opening zip file|ZipFile invalid").find(text) {
        let jar = rx!(r"([\w.\-+]+\.jar)").find(&text[m.start().saturating_sub(300)..(m.end() + 300).min(text.len())]).map(|j| j.as_str().to_string());
        out.push(finding(
            "corrupt-file",
            "A downloaded file is damaged",
            format!("Java couldn't read {} because the file is incomplete or damaged.", jar.clone().map(|j| format!("\"{j}\"")).unwrap_or_else(|| "a .jar file".into())),
            vec!["Use Repair on this Save to replace damaged game files.".into(), "If it's a mod you added, remove it and download it again.".into()],
            "high",
            Some(line_of(text, m)),
            vec![],
        ));
    }

    if let Some(m) = rx!(r"(?i)There is not enough space on the disk|No space left on device").find(text) {
        out.push(finding(
            "disk-full",
            "Your drive is full",
            "Minecraft couldn't write a file because there's no free space on the drive.",
            vec!["Free some space (Settings → Advanced → Clear downloaded game files frees shared downloads), then try again.".into()],
            "high",
            Some(line_of(text, m)),
            vec![],
        ));
    }

    if let Some(m) = rx!(r"Couldn't load chunk|Failed to read (?:region|chunk)|Chunk file at \S+ is in the wrong location|Exception reading \S*level\.dat|Failed to load level|Couldn't load level|Error reading chunk|RegionFileStorage|Failed to load data for chunk|incorrect header check").find(text) {
        out.push(finding(
            "world-damaged",
            "A world's data looks damaged",
            "Minecraft couldn't read part of a world (a chunk or the level file). This can follow a crash or power loss while the world was saving.",
            vec![
                "Restore the world from a backup in this Save's Backups tab. Restoring adds a copy and never overwrites the damaged world.".into(),
                "Open a different world to check whether the problem is only this one.".into(),
            ],
            "medium",
            Some(line_of(text, m)),
            vec![],
        ));
    }

    if let Some(m) = rx!(r"(?i)(?:failed to (?:load|compile|link) (?:shader|program)|ShaderCompileException|ShaderLinkException|error while (?:loading|reloading) shader ?pack|iris[^\n]{0,80}(?:exception|failed))").find(text) {
        out.push(finding(
            "shaders",
            "A shader pack failed",
            "The shader pack in use couldn't be compiled for your graphics card or isn't compatible with this Iris version.",
            vec!["Turn the shader pack off (Video Settings → Shader Packs → OFF), or remove it from the shaderpacks folder.".into(), "Try a different shader pack, or update Iris.".into()],
            "medium",
            Some(line_of(text, m)),
            vec!["iris".into()],
        ));
    }

    if out.is_empty() {
        if let Some(c) = rx!(r"(?:NoClassDefFoundError|ClassNotFoundException):\s*([\w/$.]+)").captures(text) {
            let class = c[1].replace('/', ".");
            out.push(finding(
                "missing-class",
                "A mod is missing something it needs",
                format!("A mod tried to use {class}, which isn't there. Usually a required library mod is missing, or the mod was made for a different Minecraft version."),
                vec!["Check the Mods tab for missing requirements.".into(), format!("Make sure every mod is made for Minecraft {mc}."), "If you can't tell which mod it is, use Find the broken mod.".into()],
                "medium",
                Some(line_of(text, c.get(0).unwrap())),
                vec![],
            ));
        } else if let Some(m) = rx!(r"NoSuchMethodError|NoSuchFieldError|AbstractMethodError|IncompatibleClassChangeError").find(text) {
            out.push(finding(
                "wrong-version",
                "A mod was made for a different version",
                format!("A mod called code that has changed. It was probably made for another Minecraft version than {mc}, or for another version of a mod it depends on."),
                vec![format!("Update your mods to builds for Minecraft {mc}."), "Use Find the broken mod to see which one it is.".into()],
                "medium",
                Some(line_of(text, m)),
                vec![],
            ));
        } else if let Some(m) = rx!(r"StackOverflowError").find(text) {
            out.push(finding(
                "stack-overflow",
                "Something got stuck in a loop",
                "The game kept calling the same code until it ran out of room. This is usually two mods changing the same thing.",
                vec!["Use Find the broken mod to find the mods involved.".into()],
                "medium",
                Some(line_of(text, m)),
                vec![],
            ));
        }
    }

    // Exit codes Windows uses when it ends a process.
    if out.is_empty() {
        match exit_code {
            Some(-1073741819) | Some(-1073740791) | Some(-1073740940) => out.push(finding(
                "native-exit",
                "Minecraft crashed outside Java",
                format!("Windows ended Minecraft after a native crash (exit code {}). Graphics drivers and overlays are the usual cause.", exit_code.unwrap()),
                vec!["Update your graphics driver.".into(), "Turn off overlays and screen recorders.".into(), "If it keeps happening, use Find the broken mod.".into()],
                "medium",
                None,
                vec![],
            )),
            Some(-805306369) => out.push(finding(
                "not-responding",
                "Minecraft stopped responding and was closed",
                "Windows closed the game because it stopped responding (exit code -805306369). This can be a freeze in a mod, too little memory, or a driver hang.",
                vec!["Give the Save a little more memory and try again.".into(), "If it froze in the same place, use Find the broken mod.".into()],
                "medium",
                None,
                vec![],
            )),
            _ => {}
        }
    }

    let trace = mods_in_trace(text, ctx);
    let named: BTreeSet<&String> = out.iter().flat_map(|f| f.mods.iter()).collect();
    let extra: Vec<String> = trace.iter().filter(|m| !named.contains(m)).cloned().collect();
    if !extra.is_empty() && !out.iter().any(|f| f.id == "fabric-incompatible" || f.id == "manual") {
        let names: Vec<String> = extra.iter().map(|m| mod_label(ctx, m)).collect();
        out.push(finding(
            "mods-in-trace",
            format!("{} {} running when it crashed", names.join(", "), if names.len() == 1 { "was" } else { "were" }),
            "These mods' code appears in the error's stack trace. That doesn't prove they caused it, but they're the first to check.",
            vec![format!("Update {}.", names.join(", ")), "Or disable them one at a time, or use Find the broken mod.".into()],
            if out.is_empty() { "medium" } else { "low" },
            None,
            extra,
        ));
    }

    if out.is_empty() {
        let head = headline(text);
        out.push(finding(
            "unknown",
            "Stabled couldn't tell what caused this crash",
            match &head {
                Some(h) => format!("Minecraft reported: {h}. None of Stabled's known causes match it."),
                None => "The game closed with an error, but the logs don't say why.".into(),
            },
            vec![
                "Use Find the broken mod: Stabled tests your mods in a copy of this Save to find the one causing it.".into(),
                "Use Repair to rule out damaged game files.".into(),
                "Ask in the Stabled Discord and share the crash report from the Logs tab.".into(),
            ],
            "low",
            head,
            vec![],
        ));
    }

    let rank = |c: &str| match c {
        "high" => 0,
        "medium" => 1,
        _ => 2,
    };
    out.sort_by_key(|f| rank(&f.confidence));
    out
}

/// Whether a log or report shows the game crashing (as opposed to errors Minecraft logs and carries on from).
pub fn looks_like_crash(text: &str) -> bool {
    rx!(r"---- Minecraft Crash Report|#@!@# Game crashed|A fatal error has been detected by the Java Runtime|/FATAL\]|Incompatible mods? (?:set )?found|Some of your mods are incompatible|Exception in thread .main.|OutOfMemoryError|Could not reserve enough space|Error occurred during initialization of VM|MixinApplyError|Could not execute entrypoint stage|Pixel format not accelerated|Manually triggered debug crash").is_match(text)
}

pub fn report(text: &str, file: Option<String>, exit_code: Option<i32>, ctx: &Context) -> CrashReport {
    let crashed = exit_code.map(|c| c != 0).unwrap_or(false) || looks_like_crash(text);
    if !crashed {
        // An ordinary log: say so instead of explaining a crash that didn't happen.
        let errors = rx!(r"(?m)/(?:ERROR|FATAL)\]").find_iter(text).count();
        let offline = rx!(r"UnknownHostException|Failed to (?:fetch|verify) (?:user properties|.*profile)|Services are unavailable|Couldn't connect to realms|Could not connect to Realms").is_match(text);
        let mut fixes = vec!["If something went wrong in game, open the crash report (if there is one) or check the lines marked as errors.".to_string()];
        if offline {
            fixes.insert(0, "Errors about Mojang, Realms or skin servers mean Minecraft couldn't reach them (no internet, or the servers were busy). They don't affect singleplayer.".into());
        }
        let finding = finding(
            "no-crash",
            "This log doesn't show a crash",
            if errors == 0 {
                "The game started and closed normally, and nothing was logged as an error.".to_string()
            } else {
                format!("The game kept running. It logged {errors} error line{}, which Minecraft often does for problems it recovers from.", if errors == 1 { "" } else { "s" })
            },
            fixes,
            "high",
            None,
            vec![],
        );
        return CrashReport { file, exit_code, headline: None, findings: vec![finding], mods_in_trace: vec![] };
    }
    CrashReport { file, exit_code, headline: headline(text), findings: analyze(text, exit_code, ctx), mods_in_trace: mods_in_trace(text, ctx) }
}

fn newer_files(dir: &Path, since: SystemTime, keep: impl Fn(&str) -> bool) -> Vec<(PathBuf, SystemTime)> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| keep(&e.file_name().to_string_lossy()))
        .filter_map(|e| {
            let t = e.metadata().ok()?.modified().ok()?;
            (t >= since).then(|| (e.path(), t))
        })
        .collect()
}

/// After the game exits: the crash report written during this session (or the JVM crash file, or the logs) and its
/// analysis. Returns None when the session ended normally.
pub fn after_exit(meta: &SaveMeta, started: SystemTime, exit_code: Option<i32>, ctx: &Context) -> Option<CrashReport> {
    let since = started.checked_sub(std::time::Duration::from_secs(2)).unwrap_or(started);
    let game = meta.game_dir();
    let mut candidates = newer_files(&game.join("crash-reports"), since, |n| n.ends_with(".txt"));
    candidates.extend(newer_files(&game, since, |n| n.starts_with("hs_err_pid") && n.ends_with(".log")));
    candidates.sort_by_key(|(_, t)| *t);
    let crashed = exit_code.map(|c| c != 0).unwrap_or(false) || !candidates.is_empty();
    if !crashed {
        return None;
    }
    if let Some((path, _)) = candidates.last() {
        let text = crate::logs::tail(path, 2 * 1024 * 1024).map(|(b, _)| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
        let rel = path.strip_prefix(&game).map(|p| p.to_string_lossy().replace('\\', "/")).ok();
        let mut r = report(&text, rel, exit_code, ctx);
        // The log can add context the crash report lacks (e.g. Fabric's dependency messages).
        if r.findings.iter().all(|f| f.confidence == "low") {
            let log = log_text(meta);
            let more = analyze(&log, exit_code, ctx);
            if more.iter().any(|f| f.confidence != "low") {
                r.findings = more;
            }
        }
        return Some(r);
    }
    let text = log_text(meta);
    Some(report(&text, Some("logs/latest.log".into()), exit_code, ctx))
}

/// The game log plus Stabled's capture of console output (Fabric prints some errors before logging starts).
fn log_text(meta: &SaveMeta) -> String {
    let logs = meta.game_dir().join("logs");
    let mut text = String::new();
    for (name, max) in [("latest.log", 1024 * 1024), ("stabled-launch.log", 512 * 1024)] {
        if let Ok((b, _)) = crate::logs::tail(&logs.join(name), max) {
            text.push_str(&String::from_utf8_lossy(&b));
            text.push('\n');
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(mods: &HashMap<String, String>) -> Context<'_> {
        Context { minecraft: "1.21.1", memory_mb: 4096, mods }
    }

    #[test]
    fn explains_common_crashes() {
        let mods: HashMap<String, String> = [("sodium".to_string(), "Sodium".to_string()), ("examplemod".to_string(), "Example Mod".to_string())].into();
        let c = ctx(&mods);

        let oom = analyze("java.lang.OutOfMemoryError: Java heap space\n\tat foo", Some(-1), &c);
        assert_eq!(oom[0].id, "out-of-memory");
        assert_eq!(oom[0].confidence, "high");

        let fabric = "Incompatible mods found!\nnet.fabricmc.loader.impl.FormattedException: Some of your mods are incompatible with the game or each other!\nA potential solution has been determined, this may resolve your problem:\n\t - Replace mod 'Sodium' (sodium) 0.5.3 with version 0.5.8 or later.\nMore details:\n\t - Mod 'Iris' (iris) 1.7.0 requires version 0.5.8 or later of mod 'Sodium' (sodium), but only the wrong version is present: 0.5.3!\n";
        let f = analyze(fabric, Some(1), &c);
        assert_eq!(f[0].id, "fabric-incompatible");
        assert!(f[0].fixes[0].starts_with("Replace mod 'Sodium'"));
        assert!(f[0].mods.contains(&"sodium".to_string()));

        let mixin = "Caused by: org.spongepowered.asm.mixin.throwables.MixinApplyError: Mixin [examplemod.mixins.json:FooMixin from mod examplemod] from phase [DEFAULT] in config [examplemod.mixins.json] FAILED during APPLY";
        let m = analyze(mixin, Some(-1), &c);
        assert_eq!(m[0].id, "mixin");
        assert!(m[0].title.contains("Example Mod"));

        let driver = "# A fatal error has been detected by the Java Runtime Environment:\n#  EXCEPTION_ACCESS_VIOLATION (0xc0000005) at pc=0x0000\n# Problematic frame:\n# C  [atio6axx.dll+0x1234]";
        let d = analyze(driver, Some(-1073741819), &c);
        assert_eq!(d[0].id, "driver-crash");
        assert!(d[0].title.contains("AMD"));

        let trace = "java.lang.NullPointerException: Cannot invoke \"Object.toString()\"\n\tat net.minecraft.class_310.handler$zzb000$sodium$onTick(MinecraftClient.java:100)";
        let t = analyze(trace, Some(-1), &c);
        assert!(t.iter().any(|f| f.id == "mods-in-trace" && f.mods == vec!["sodium".to_string()]));

        let normal = report("[05:01:21] [Render thread/ERROR]: Failed to fetch user properties\njava.net.UnknownHostException: sessionserver.mojang.com\n", None, None, &c);
        assert_eq!(normal.findings[0].id, "no-crash");
        assert!(looks_like_crash("---- Minecraft Crash Report ----\n"));

        let unknown = analyze("Description: Ticking entity\n\njava.lang.IllegalStateException: odd\n", Some(-1), &c);
        assert_eq!(unknown[0].confidence, "low");
        assert_eq!(headline("Description: Ticking entity\n\njava.lang.IllegalStateException: odd\n").as_deref(), Some("Ticking entity: java.lang.IllegalStateException: odd"));
    }
}
