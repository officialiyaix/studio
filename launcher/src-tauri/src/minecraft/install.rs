//! Installs everything a Save needs: Mojang's game files and Java runtime, the Fabric loader, and the managed mods
//! (the bundled Stabled client plus the pinned performance stack). Shared, immutable downloads live in the shared
//! cache; only the Save's own `mods` folder receives copies, so each Save stays independent.

use super::{java, rules, stack};
use crate::bail;
use crate::error::AppResult;
use crate::paths;
use crate::saves::{managed, SaveMeta};
use crate::util::download::{self, BatchProgress, Download, Hash};
use crate::util::fsx;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub phase: String,
    pub label: String,
    pub files_done: usize,
    pub files_total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

/// Everything the launch step needs, resolved for this platform.
pub struct Prepared {
    pub version_json: Value,
    pub fabric_profile: Value,
    pub classpath: Vec<PathBuf>,
    pub java: PathBuf,
    pub asset_index: String,
    pub logging_arg: Option<String>,
}

fn version_dir(id: &str) -> PathBuf {
    paths::versions_dir().join(id)
}

fn marker(stack: &stack::Stack) -> PathBuf {
    paths::versions_dir().join(format!("installed-{}.json", stack.key().replace('@', "-r")))
}

async fn load_or_fetch_json(url: &str, path: &PathBuf, sha1: Option<&str>) -> AppResult<Value> {
    let valid = match sha1 {
        Some(h) => download::is_valid(path, &Hash::Sha1(h.to_string()), None),
        None => path.is_file(),
    };
    if !valid {
        download::fetch(&Download {
            url: url.to_string(),
            path: path.clone(),
            hash: sha1.map(|h| Hash::Sha1(h.to_string())).unwrap_or(Hash::None),
            size: None,
        }, None)
        .await?;
    }
    fsx::read_json(path)
}

pub async fn prepare(save: &SaveMeta, concurrency: usize, emit: &(dyn Fn(Progress) + Send + Sync)) -> AppResult<Prepared> {
    let Some(stack) = stack::for_version(&save.minecraft_version) else {
        bail!("UNSUPPORTED", "Minecraft {} is not supported by the Stabled client", save.minecraft_version);
    };
    // A full hash check runs once per stack revision; afterwards launches only check presence and size.
    let full_check = !marker(stack).is_file();
    let step = |phase: &str, label: &str| Progress {
        phase: phase.into(),
        label: label.into(),
        files_done: 0,
        files_total: 0,
        bytes_done: 0,
        bytes_total: 0,
    };

    emit(step("metadata", "Reading game metadata"));
    let (url, sha1) = super::manifest::version_url(&stack.minecraft).await?;
    let vdir = version_dir(&stack.minecraft);
    let version_json = load_or_fetch_json(&url, &vdir.join(format!("{}.json", stack.minecraft)), Some(&sha1)).await?;
    let profile_id = stack.fabric_profile_id();
    let profile_url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        stack.minecraft, stack.loader.version
    );
    let fabric_profile = load_or_fetch_json(&profile_url, &version_dir(&profile_id).join(format!("{profile_id}.json")), None).await?;

    // ---- Collect downloads ----
    let mut downloads: Vec<Download> = Vec::new();
    let mut classpath: Vec<PathBuf> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Fabric libraries come first so they take precedence over vanilla copies of the same artifact.
    for lib in fabric_profile["libraries"].as_array().into_iter().flatten() {
        let name = lib["name"].as_str().unwrap_or_default();
        let Some(rel) = rules::maven_path(name) else { continue };
        if !seen.insert(rules::library_key(name)) {
            continue;
        }
        let base = lib["url"].as_str().unwrap_or("https://maven.fabricmc.net/");
        let path = paths::libraries_dir().join(&rel);
        downloads.push(Download {
            url: format!("{}{}", base.trim_end_matches('/').to_string() + "/", rel),
            path: path.clone(),
            hash: lib["sha1"].as_str().map(|h| Hash::Sha1(h.into())).unwrap_or(Hash::None),
            size: lib["size"].as_u64(),
        });
        classpath.push(path);
    }
    for lib in version_json["libraries"].as_array().into_iter().flatten() {
        let name = lib["name"].as_str().unwrap_or_default();
        if !rules::allowed(lib.get("rules"), &[]) || !rules::library_applies_to_arch(name) {
            continue;
        }
        if !seen.insert(rules::library_key(name)) {
            continue;
        }
        let art = &lib["downloads"]["artifact"];
        let Some(rel) = art["path"].as_str().map(str::to_owned).or_else(|| rules::maven_path(name)) else { continue };
        let Some(target) = fsx::contained(&paths::libraries_dir(), &rel) else { continue };
        downloads.push(Download {
            url: art["url"].as_str().unwrap_or_default().to_string(),
            path: target.clone(),
            hash: art["sha1"].as_str().map(|h| Hash::Sha1(h.into())).unwrap_or(Hash::None),
            size: art["size"].as_u64(),
        });
        classpath.push(target);
    }
    let client = &version_json["downloads"]["client"];
    let client_jar = vdir.join(format!("{}.jar", stack.minecraft));
    downloads.push(Download {
        url: client["url"].as_str().unwrap_or_default().into(),
        path: client_jar.clone(),
        hash: Hash::Sha1(client["sha1"].as_str().unwrap_or_default().into()),
        size: client["size"].as_u64(),
    });
    classpath.push(client_jar);

    let mut logging_arg = None;
    if let Some(logging) = version_json["logging"]["client"].as_object() {
        let file = &logging["file"];
        if let Some(id) = file["id"].as_str() {
            let path = paths::assets_dir().join("log_configs").join(id);
            downloads.push(Download {
                url: file["url"].as_str().unwrap_or_default().into(),
                path: path.clone(),
                hash: Hash::Sha1(file["sha1"].as_str().unwrap_or_default().into()),
                size: file["size"].as_u64(),
            });
            logging_arg = logging["argument"].as_str().map(|a| a.replace("${path}", &path.to_string_lossy()));
        }
    }

    // ---- Assets ----
    emit(step("metadata", "Reading asset index"));
    let asset_index = version_json["assetIndex"]["id"].as_str().unwrap_or_default().to_string();
    let index_path = paths::assets_dir().join("indexes").join(format!("{asset_index}.json"));
    let index = load_or_fetch_json(
        version_json["assetIndex"]["url"].as_str().unwrap_or_default(),
        &index_path,
        version_json["assetIndex"]["sha1"].as_str(),
    )
    .await?;
    let mut asset_downloads = Vec::new();
    for (_, obj) in index["objects"].as_object().into_iter().flatten() {
        let Some(hash) = obj["hash"].as_str() else { continue };
        if hash.len() < 2 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        asset_downloads.push(Download {
            url: format!("https://resources.download.minecraft.net/{}/{}", &hash[..2], hash),
            path: paths::assets_dir().join("objects").join(&hash[..2]).join(hash),
            hash: Hash::Sha1(hash.to_string()),
            size: obj["size"].as_u64(),
        });
    }

    // ---- Java ----
    emit(step("java", "Preparing Java"));
    let java_exe = java::executable(&stack.java.component);
    let mut java_downloads = Vec::new();
    let mut java_execs = Vec::new();
    if full_check || !java_exe.is_file() {
        let (files, execs) = java::plan(&stack.java.component).await?;
        java_downloads = files;
        java_execs = execs;
    }

    // ---- Run downloads ----
    let quick = |items: Vec<Download>| -> Vec<Download> {
        if full_check {
            items
        } else {
            // Size-only check on later launches; anything missing or truncated is re-downloaded and fully verified.
            items
                .into_iter()
                .filter(|d| std::fs::metadata(&d.path).map(|m| d.size.map(|s| s != m.len()).unwrap_or(m.len() == 0)).unwrap_or(true))
                .collect()
        }
    };
    for (phase, label, items) in [
        ("java", "Downloading Java", quick(java_downloads)),
        ("libraries", "Downloading game files", quick(downloads)),
        ("assets", "Downloading assets", quick(asset_downloads)),
    ] {
        run_batch(phase, label, items, concurrency, emit).await?;
    }
    java::mark_executable(&java_execs);
    if !java_exe.is_file() {
        bail!("JAVA", "The Java runtime could not be installed");
    }

    // ---- Managed mods (per Save copy, verified) ----
    emit(step("mods", "Installing Stabled components"));
    managed::sync(save, stack, concurrency).await?;

    if full_check {
        let _ = fsx::write_json(&marker(stack), &serde_json::json!({ "stack": stack.key(), "at": chrono::Utc::now().to_rfc3339() }));
    }
    Ok(Prepared { version_json, fabric_profile, classpath, java: java_exe, asset_index, logging_arg })
}

async fn run_batch(
    phase: &str,
    label: &str,
    items: Vec<Download>,
    concurrency: usize,
    emit: &(dyn Fn(Progress) + Send + Sync),
) -> AppResult<()> {
    // Several entries can name the same file (asset indexes list identical objects under different names);
    // downloading one path twice in parallel would race on its temporary file.
    let mut seen = std::collections::HashSet::new();
    let items: Vec<Download> = items.into_iter().filter(|d| seen.insert(d.path.clone())).collect();
    if items.is_empty() {
        return Ok(());
    }
    let total_files = items.len();
    let total_bytes: u64 = items.iter().filter_map(|d| d.size).sum();
    let progress = Arc::new(BatchProgress { files_done: Default::default(), bytes_done: Default::default() });
    let task = {
        let progress = progress.clone();
        tokio::spawn(async move { download::fetch_all(items, concurrency, progress).await })
    };
    loop {
        let done = task.is_finished();
        emit(Progress {
            phase: phase.into(),
            label: label.into(),
            files_done: progress.files_done.load(Ordering::Relaxed),
            files_total: total_files,
            bytes_done: progress.bytes_done.load(Ordering::Relaxed),
            bytes_total: total_bytes,
        });
        if done {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    task.await.map_err(|e| crate::error::AppError::new("DOWNLOAD", e.to_string()))?
}

pub fn is_installed(save: &SaveMeta) -> bool {
    let Some(stack) = stack::for_version(&save.minecraft_version) else { return false };
    marker(stack).is_file() && java::runtime_version_ok(&stack.java.component) && save.installed_stack.as_deref() == Some(stack.key().as_str())
}
