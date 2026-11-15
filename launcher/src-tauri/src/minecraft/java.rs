//! Java runtime from Mojang's official runtime manifest (the same runtimes the official launcher uses), shared by
//! all Saves that need the same component.

use crate::bail;
use crate::error::AppResult;
use crate::paths;
use crate::util::download::{self, Download, Hash};
use serde_json::Value;
use std::path::PathBuf;

const RUNTIME_INDEX: &str = "https://piston-meta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

fn platform() -> &'static str {
    if cfg!(target_os = "windows") {
        if cfg!(target_arch = "aarch64") { "windows-arm64" } else { "windows-x64" }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") { "mac-os-arm64" } else { "mac-os" }
    } else {
        "linux"
    }
}

pub fn runtime_dir(component: &str) -> PathBuf {
    paths::runtimes_dir().join(component).join(platform())
}

pub fn executable(component: &str) -> PathBuf {
    let dir = runtime_dir(component);
    if cfg!(target_os = "windows") {
        dir.join("bin").join("javaw.exe")
    } else if cfg!(target_os = "macos") {
        dir.join("jre.bundle/Contents/Home/bin/java")
    } else {
        dir.join("bin").join("java")
    }
}

/// Returns the download list for a runtime component (files only; directories are created on the way).
pub async fn plan(component: &str) -> AppResult<(Vec<Download>, Vec<PathBuf>)> {
    let index = download::get_json(RUNTIME_INDEX).await?;
    let entries = index[platform()][component].as_array().cloned().unwrap_or_default();
    let Some(entry) = entries.first() else {
        bail!("JAVA", "Mojang does not publish the {component} Java runtime for this platform");
    };
    let manifest_url = entry["manifest"]["url"].as_str().unwrap_or_default();
    let manifest = download::get_json(manifest_url).await?;
    let root = runtime_dir(component);
    let mut files = Vec::new();
    let mut executables = Vec::new();
    for (rel, info) in manifest["files"].as_object().into_iter().flatten() {
        if info["type"].as_str() != Some("file") {
            continue;
        }
        let Some(target) = crate::util::fsx::contained(&root, rel) else { continue };
        let raw = &info["downloads"]["raw"];
        files.push(Download {
            url: raw["url"].as_str().unwrap_or_default().to_string(),
            path: target.clone(),
            hash: Hash::Sha1(raw["sha1"].as_str().unwrap_or_default().to_string()),
            size: raw["size"].as_u64(),
        });
        if info["executable"].as_bool().unwrap_or(false) {
            executables.push(target);
        }
    }
    Ok((files, executables))
}

pub fn mark_executable(paths: &[PathBuf]) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for p in paths {
            if let Ok(meta) = std::fs::metadata(p) {
                let mut perm = meta.permissions();
                perm.set_mode(0o755);
                let _ = std::fs::set_permissions(p, perm);
            }
        }
    }
    #[cfg(not(unix))]
    let _ = paths;
}

pub fn runtime_version_ok(component: &str) -> bool {
    executable(component).is_file()
}

#[allow(dead_code)]
pub fn describe(v: &Value) -> String {
    v.to_string()
}
