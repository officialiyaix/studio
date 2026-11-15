//! Per-Save mod management: list, add, enable/disable, remove, and detectable compatibility problems.
//!
//! Detection covers what Fabric metadata states: wrong loader, duplicate mod ids, Minecraft version ranges, missing
//! dependencies and declared `breaks`. It cannot predict every runtime incompatibility, and the UI says so.

use crate::bail;
use crate::error::AppResult;
use crate::saves::{managed, SaveMeta};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModEntry {
    pub file: String,
    pub enabled: bool,
    pub mod_id: Option<String>,
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub loader: String,
    pub size: u64,
    pub managed: Option<managed::ManagedInfo>,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// "error" (will stop the game from starting or the mod from loading) or "warning".
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModList {
    pub mods: Vec<ModEntry>,
    pub issues: Vec<Issue>,
    /// Optional Stabled components the user removed; they can be restored.
    pub removed: Vec<managed::RemovedComponent>,
}

struct Parsed {
    id: Option<String>,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    loader: &'static str,
    depends: HashMap<String, Value>,
    breaks: HashMap<String, Value>,
    provides: Vec<String>,
}

fn parse_fabric(json: &Value) -> Parsed {
    let map = |key: &str| -> HashMap<String, Value> {
        json[key].as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default()
    };
    Parsed {
        id: json["id"].as_str().map(str::to_owned),
        name: json["name"].as_str().map(str::to_owned),
        version: json["version"].as_str().map(str::to_owned),
        description: json["description"].as_str().map(|d| d.chars().take(240).collect()),
        loader: "fabric",
        depends: map("depends"),
        breaks: map("breaks"),
        provides: json["provides"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()).unwrap_or_default(),
    }
}

/// Reads `fabric.mod.json` (and the ids of jars nested inside it) without extracting the jar.
fn inspect_jar(path: &Path) -> Option<(Parsed, Vec<String>)> {
    let file = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    let mut nested_ids = Vec::new();
    let fabric_text = match zip.by_name("fabric.mod.json") {
        Ok(mut f) => {
            let mut text = String::new();
            f.read_to_string(&mut text).ok()?;
            Some(text)
        }
        Err(_) => None,
    };
    let parsed = if let Some(text) = fabric_text {
        // Some mods ship fabric.mod.json with comment lines; tolerate them.
        let json: Value = serde_json::from_str(&text).or_else(|_| serde_json::from_str(&strip_json_comments(&text))).ok()?;
        let nested: Vec<String> = json["jars"].as_array().into_iter().flatten().filter_map(|j| j["file"].as_str().map(str::to_owned)).collect();
        for n in nested {
            let bytes = match zip.by_name(&n) {
                Ok(mut inner) => {
                    let mut b = Vec::new();
                    if inner.read_to_end(&mut b).is_err() {
                        continue;
                    }
                    b
                }
                Err(_) => continue,
            };
            let Ok(mut inner_zip) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) else { continue };
            let text = match inner_zip.by_name("fabric.mod.json") {
                Ok(mut fmj) => {
                    let mut t = String::new();
                    if fmj.read_to_string(&mut t).is_err() {
                        continue;
                    }
                    t
                }
                Err(_) => continue,
            };
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                if let Some(id) = v["id"].as_str() {
                    nested_ids.push(id.to_string());
                }
                for p in v["provides"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                    nested_ids.push(p.to_string());
                }
            }
        }
        parse_fabric(&json)
    } else {
        let has = |zip: &mut zip::ZipArchive<std::fs::File>, name: &str| zip.by_name(name).is_ok();
        let loader = if has(&mut zip, "quilt.mod.json") {
            "quilt"
        } else if has(&mut zip, "META-INF/neoforge.mods.toml") {
            "neoforge"
        } else if has(&mut zip, "META-INF/mods.toml") {
            "forge"
        } else {
            "unknown"
        };
        Parsed { id: None, name: None, version: None, description: None, loader, depends: HashMap::new(), breaks: HashMap::new(), provides: vec![] }
    };
    Some((parsed, nested_ids))
}

fn strip_json_comments(s: &str) -> String {
    s.lines().map(|l| if l.trim_start().starts_with("//") { "" } else { l }).collect::<Vec<_>>().join("\n")
}

pub fn list(meta: &SaveMeta) -> AppResult<ModList> {
    let dir = meta.mods_dir();
    std::fs::create_dir_all(&dir)?;
    let managed_state = managed::load(meta);
    let mut entries: Vec<(ModEntry, Option<Parsed>, Vec<String>)> = Vec::new();
    for e in std::fs::read_dir(&dir)?.flatten() {
        let path = e.path();
        let file = e.file_name().to_string_lossy().into_owned();
        let enabled = file.ends_with(".jar");
        if !(enabled || file.ends_with(".jar.disabled")) || !path.is_file() {
            continue;
        }
        let size = e.metadata().map(|m| m.len()).unwrap_or(0);
        let inspected = inspect_jar(&path);
        let (parsed, nested) = match inspected {
            Some((p, n)) => (Some(p), n),
            None => (None, vec![]),
        };
        let base_name = file.trim_end_matches(".disabled").to_string();
        let managed_info = managed::info_for(&managed_state, &base_name);
        let mut entry = ModEntry {
            file: file.clone(),
            enabled,
            mod_id: parsed.as_ref().and_then(|p| p.id.clone()),
            name: parsed.as_ref().and_then(|p| p.name.clone()).unwrap_or_else(|| base_name.trim_end_matches(".jar").to_string()),
            version: parsed.as_ref().and_then(|p| p.version.clone()),
            description: parsed.as_ref().and_then(|p| p.description.clone()),
            loader: parsed.as_ref().map(|p| p.loader.to_string()).unwrap_or_else(|| "unknown".into()),
            size,
            managed: managed_info,
            issues: vec![],
        };
        if parsed.is_none() {
            entry.issues.push(Issue { level: "error".into(), message: "This file is not a valid mod archive.".into() });
        }
        entries.push((entry, parsed, nested));
    }

    // Everything the enabled set provides: mod ids, `provides` aliases and nested jars, plus built-ins.
    let mut provided: HashSet<String> = ["minecraft", "java", "fabricloader", "fabric-loader", "mixinextras"].iter().map(|s| s.to_string()).collect();
    let mut owners: HashMap<String, Vec<String>> = HashMap::new();
    let mut versions: HashMap<String, String> = HashMap::new();
    for (e, p, nested) in &entries {
        if !e.enabled {
            continue;
        }
        if let Some(p) = p {
            if let Some(id) = &p.id {
                provided.insert(id.clone());
                owners.entry(id.clone()).or_default().push(e.file.clone());
                if let Some(v) = &p.version {
                    versions.insert(id.clone(), v.clone());
                }
            }
            provided.extend(p.provides.iter().cloned());
            provided.extend(nested.iter().cloned());
        }
    }
    let mc = meta.minecraft_version.clone();
    let mut global = Vec::new();
    for (id, files) in &owners {
        if files.len() > 1 {
            global.push(Issue {
                level: "error".into(),
                message: format!("{} files provide the mod \"{id}\" ({}). Fabric will refuse to start until one is disabled.", files.len(), files.join(", ")),
            });
        }
    }
    let enabled_ids: HashSet<String> = owners.keys().cloned().collect();
    for (e, p, _) in entries.iter_mut() {
        let Some(p) = p else { continue };
        match p.loader {
            "fabric" => {}
            "quilt" => e.issues.push(Issue { level: "error".into(), message: "This is a Quilt mod. Stabled Saves use Fabric, so it will not load.".into() }),
            "forge" | "neoforge" => e.issues.push(Issue { level: "error".into(), message: format!("This is a {} mod. Stabled Saves use Fabric, so it will not load.", if p.loader == "forge" { "Forge" } else { "NeoForge" }) }),
            _ => e.issues.push(Issue { level: "error".into(), message: "No Fabric mod metadata was found in this file.".into() }),
        }
        if !e.enabled {
            continue;
        }
        if let Some(pred) = p.depends.get("minecraft") {
            if !predicate_matches(pred, &mc) {
                e.issues.push(Issue { level: "error".into(), message: format!("Made for Minecraft {}, but this Save uses {mc}.", describe(pred)) });
            }
        }
        for (dep, _) in &p.depends {
            if !provided.contains(dep) && !dep.starts_with("java") {
                e.issues.push(Issue { level: "error".into(), message: format!("Requires \"{dep}\", which is not installed or is disabled.") });
            }
        }
        for (other, pred) in &p.breaks {
            if !enabled_ids.contains(other) {
                continue;
            }
            // `breaks` usually targets a version range (e.g. old releases); only report it when the installed one matches.
            let installed = versions.get(other).map(String::as_str).unwrap_or("");
            if installed.is_empty() || predicate_matches(pred, installed) {
                let which = if installed.is_empty() { String::new() } else { format!(" {installed}") };
                e.issues.push(Issue { level: "error".into(), message: format!("Declares that it is incompatible with \"{other}\"{which}, which is enabled.") });
            }
        }
    }
    for m in managed::skipped(&managed_state) {
        global.push(Issue { level: "warning".into(), message: m });
    }
    let mut mods: Vec<ModEntry> = entries.into_iter().map(|(e, _, _)| e).collect();
    mods.sort_by(|a, b| b.managed.is_some().cmp(&a.managed.is_some()).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(ModList { mods, issues: global, removed: managed::removed(&managed_state) })
}

fn mod_path(meta: &SaveMeta, file: &str) -> AppResult<PathBuf> {
    let Some(name) = crate::util::fsx::safe_file_name(file) else { bail!("MODS", "Invalid file name") };
    if !(name.ends_with(".jar") || name.ends_with(".jar.disabled")) {
        bail!("MODS", "Only .jar mod files can be managed here");
    }
    let p = meta.mods_dir().join(&name);
    if !p.is_file() {
        bail!("MODS", "That mod file no longer exists");
    }
    Ok(p)
}

/// Copy local jars into this Save only. Existing files are never overwritten.
pub fn add(meta: &SaveMeta, sources: &[String]) -> AppResult<Vec<String>> {
    let mut added = Vec::new();
    for src in sources {
        let src = Path::new(src);
        let Some(name) = src.file_name().map(|n| n.to_string_lossy().into_owned()) else { continue };
        let Some(name) = crate::util::fsx::safe_file_name(&name) else { bail!("MODS", "\"{}\" has a file name Windows can't use", src.display()) };
        if !name.to_ascii_lowercase().ends_with(".jar") {
            bail!("MODS", "\"{name}\" is not a .jar file");
        }
        if inspect_jar(src).is_none() {
            bail!("MODS", "\"{name}\" is not a valid mod archive");
        }
        let target = meta.mods_dir().join(&name);
        let disabled = meta.mods_dir().join(format!("{name}.disabled"));
        if target.exists() || disabled.exists() {
            bail!("MODS", "\"{name}\" is already in this Save");
        }
        std::fs::copy(src, &target)?;
        added.push(name);
    }
    Ok(added)
}

pub fn set_enabled(meta: &SaveMeta, file: &str, enabled: bool) -> AppResult<()> {
    let path = mod_path(meta, file)?;
    let base = file.trim_end_matches(".disabled").to_string();
    if !enabled && managed::is_required(&managed::load(meta), &base) {
        bail!("PROTECTED", "This component is required by the Stabled client and can't be disabled.");
    }
    let target = meta.mods_dir().join(if enabled { base.clone() } else { format!("{base}.disabled") });
    if target == path {
        return Ok(());
    }
    if target.exists() {
        bail!("MODS", "A file named {} already exists", target.file_name().unwrap().to_string_lossy());
    }
    std::fs::rename(&path, &target)?;
    managed::note_enabled(meta, &base, enabled);
    Ok(())
}

pub fn remove(meta: &SaveMeta, file: &str) -> AppResult<()> {
    let path = mod_path(meta, file)?;
    let base = file.trim_end_matches(".disabled").to_string();
    if managed::is_required(&managed::load(meta), &base) {
        bail!("PROTECTED", "This component is required by the Stabled client and can't be removed.");
    }
    trash::delete(&path).map_err(|e| crate::error::AppError::new("MODS", format!("Could not move the mod to the Recycle Bin: {e}")))?;
    managed::note_removed(meta, &base);
    Ok(())
}

pub fn read_mod_id(path: &Path) -> Option<String> {
    inspect_jar(path).and_then(|(p, _)| p.id)
}

// ---------- Find the broken mod ----------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// File name without ".disabled".
    pub file: String,
    pub name: String,
    pub mod_id: Option<String>,
    /// Files of other candidates this mod requires.
    pub requires: Vec<String>,
}

/// Mods that can be switched off for testing: every jar except the components the Stabled client needs.
/// With `enabled_only`, only currently enabled mods.
pub fn candidates(meta: &SaveMeta, enabled_only: bool) -> AppResult<Vec<Candidate>> {
    let state = managed::load(meta);
    let mut parsed: Vec<(String, Option<Parsed>, Vec<String>)> = Vec::new();
    for e in std::fs::read_dir(meta.mods_dir())?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let enabled = name.ends_with(".jar");
        if !(enabled || name.ends_with(".jar.disabled")) || (enabled_only && !enabled) {
            continue;
        }
        let base = name.trim_end_matches(".disabled").to_string();
        if managed::is_required(&state, &base) {
            continue;
        }
        let (p, nested) = match inspect_jar(&e.path()) {
            Some((p, n)) => (Some(p), n),
            None => (None, vec![]),
        };
        parsed.push((base, p, nested));
    }
    // Which candidate file provides each mod id.
    let mut provider: HashMap<String, String> = HashMap::new();
    for (file, p, nested) in &parsed {
        if let Some(p) = p {
            for id in p.id.iter().chain(p.provides.iter()).chain(nested.iter()) {
                provider.entry(id.clone()).or_insert_with(|| file.clone());
            }
        }
    }
    let mut out: Vec<Candidate> = parsed
        .iter()
        .map(|(file, p, _)| {
            let mut requires: Vec<String> = p
                .as_ref()
                .map(|p| p.depends.keys().filter_map(|d| provider.get(d)).filter(|f| *f != file).cloned().collect())
                .unwrap_or_default();
            requires.sort();
            requires.dedup();
            Candidate {
                file: file.clone(),
                name: p.as_ref().and_then(|p| p.name.clone()).unwrap_or_else(|| file.trim_end_matches(".jar").to_string()),
                mod_id: p.as_ref().and_then(|p| p.id.clone()),
                requires,
            }
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

/// In a test copy: enables `keep` (plus the mods they require) and disables the other `all` mods.
/// Returns the files that are enabled afterwards.
pub fn apply_test_set(meta: &SaveMeta, all: &[String], keep: &[String]) -> AppResult<Vec<String>> {
    let cands = candidates(meta, false)?;
    let by_file: HashMap<&str, &Candidate> = cands.iter().map(|c| (c.file.as_str(), c)).collect();
    let mut on: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = keep.to_vec();
    while let Some(f) = stack.pop() {
        if !all.contains(&f) || !on.insert(f.clone()) {
            continue;
        }
        if let Some(c) = by_file.get(f.as_str()) {
            stack.extend(c.requires.iter().cloned());
        }
    }
    for f in all {
        if !by_file.contains_key(f.as_str()) {
            continue;
        }
        let want = on.contains(f);
        let current = if meta.mods_dir().join(f).is_file() { f.clone() } else { format!("{f}.disabled") };
        set_enabled(meta, &current, want)?;
    }
    let mut v: Vec<String> = on.into_iter().collect();
    v.sort();
    Ok(v)
}

// ---------- Fabric version predicates ----------

fn describe(pred: &Value) -> String {
    match pred {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" or "),
        _ => "an unknown version".into(),
    }
}

/// Fabric dependency values are a string or an array of strings (any may match); each string is a space-separated
/// list of constraints that must all match.
pub fn predicate_matches(pred: &Value, version: &str) -> bool {
    match pred {
        Value::String(s) => constraint_set(s, version),
        Value::Array(a) => a.iter().filter_map(Value::as_str).any(|s| constraint_set(s, version)),
        _ => true,
    }
}

fn constraint_set(s: &str, version: &str) -> bool {
    s.split_whitespace().all(|c| constraint(c, version))
}

fn parse(v: &str) -> (Vec<u64>, bool) {
    let core = v.split('+').next().unwrap_or("");
    let (nums, pre) = match core.split_once('-') {
        Some((n, p)) => (n, !p.is_empty()),
        None => (core, false),
    };
    (nums.split('.').map(|p| p.parse().unwrap_or(0)).collect(), pre)
}

fn cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let (an, ap) = parse(a);
    let (bn, bp) = parse(b);
    let len = an.len().max(bn.len());
    for i in 0..len {
        let x = an.get(i).copied().unwrap_or(0);
        let y = bn.get(i).copied().unwrap_or(0);
        if x != y {
            return x.cmp(&y);
        }
    }
    bp.cmp(&ap) // a pre-release sorts before the release
}

fn constraint(c: &str, version: &str) -> bool {
    use std::cmp::Ordering::*;
    let c = c.trim();
    if c.is_empty() || c == "*" {
        return true;
    }
    let ops = [">=", "<=", ">", "<", "=", "~", "^"];
    for op in ops {
        if let Some(rest) = c.strip_prefix(op) {
            let target = rest.trim_end_matches('-');
            let o = cmp(version, target);
            let (tn, _) = parse(target);
            let (vn, _) = parse(version);
            return match op {
                ">=" => o != Less,
                "<=" => o != Greater,
                ">" => o == Greater,
                "<" => o == Less,
                "=" => o == Equal,
                "~" => o != Less && vn.first() == tn.first() && (tn.len() < 2 || vn.get(1).copied().unwrap_or(0) == tn[1]),
                "^" => o != Less && vn.first() == tn.first(),
                _ => true,
            };
        }
    }
    if c.contains('x') || c.contains('X') || c.contains('*') {
        let parts: Vec<&str> = c.split('.').collect();
        let (vn, _) = parse(version);
        for (i, p) in parts.iter().enumerate() {
            if matches!(*p, "x" | "X" | "*") {
                return true;
            }
            if vn.get(i).copied().unwrap_or(0) != p.parse::<u64>().unwrap_or(u64::MAX) {
                return false;
            }
        }
        return true;
    }
    cmp(version, c) == Equal
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn predicates_match_fabric_semantics() {
        assert!(predicate_matches(&json!("~26.3-"), "26.3"));
        assert!(predicate_matches(&json!("~26.3"), "26.3.1"));
        assert!(!predicate_matches(&json!("~26.3"), "26.4"));
        assert!(predicate_matches(&json!("26.3.x"), "26.3"));
        assert!(!predicate_matches(&json!("26.3.x"), "26.2"));
        assert!(predicate_matches(&json!(">=26.1 <27"), "26.3"));
        assert!(!predicate_matches(&json!(">=26.1 <27"), "27.1"));
        assert!(predicate_matches(&json!(["1.21.11", "26.3"]), "26.3"));
        assert!(!predicate_matches(&json!("1.21.x"), "26.3"));
        assert!(predicate_matches(&json!("*"), "26.3"));
        assert!(predicate_matches(&json!(">=0.19.3"), "0.20.1"));
        // Build metadata is ignored; pre-releases sort before releases.
        assert!(!predicate_matches(&json!("<0.150.0"), "0.166.2+26.3"));
        assert!(predicate_matches(&json!(">=0.9.3-beta.1"), "0.9.3-beta.5"));
    }
}
