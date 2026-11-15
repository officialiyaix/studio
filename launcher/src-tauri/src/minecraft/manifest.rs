//! Mojang's version manifest (cached) and the list offered by the version selector.

use crate::error::AppResult;
use crate::paths;
use crate::util::{download, fsx};
use serde::Serialize;
use serde_json::Value;

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionOption {
    pub id: String,
    pub release_time: Option<String>,
    pub supported: bool,
    pub loader: Option<String>,
    pub note: String,
    /// True when the Stabled stack for this version includes pre-release components (shown to the user).
    pub has_beta_components: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionList {
    pub versions: Vec<VersionOption>,
    /// "live", "cached" or "offline" (only the supported versions known to this build).
    pub source: String,
}

fn manifest_cache() -> std::path::PathBuf {
    paths::versions_dir().join("version_manifest_v2.json")
}

pub async fn manifest() -> AppResult<Value> {
    match download::get_json(MANIFEST_URL).await {
        Ok(v) => {
            let _ = fsx::write_json(&manifest_cache(), &v);
            Ok(v)
        }
        Err(e) => {
            if let Ok(v) = fsx::read_json::<Value>(&manifest_cache()) {
                Ok(v)
            } else {
                Err(e)
            }
        }
    }
}

pub async fn list() -> VersionList {
    let (manifest, source) = match download::get_json(MANIFEST_URL).await {
        Ok(v) => {
            let _ = fsx::write_json(&manifest_cache(), &v);
            (Some(v), "live")
        }
        Err(_) => match fsx::read_json::<Value>(&manifest_cache()) {
            Ok(v) => (Some(v), "cached"),
            Err(_) => (None, "offline"),
        },
    };
    let mut versions = Vec::new();
    if let Some(m) = manifest {
        for v in m["versions"].as_array().into_iter().flatten() {
            if v["type"].as_str() != Some("release") {
                continue;
            }
            let id = v["id"].as_str().unwrap_or_default().to_string();
            versions.push(option(&id, v["releaseTime"].as_str().map(str::to_owned)));
        }
    }
    for s in super::stack::all() {
        if !versions.iter().any(|v| v.id == s.minecraft) {
            versions.insert(0, option(&s.minecraft, None));
        }
    }
    VersionList { versions, source: source.into() }
}

fn option(id: &str, release_time: Option<String>) -> VersionOption {
    match super::stack::for_version(id) {
        Some(stack) => {
            let beta: Vec<&str> = stack.mods.iter().filter(|m| m.version_type != "release").map(|m| m.title.as_str()).collect();
            VersionOption {
                id: id.to_string(),
                release_time,
                supported: true,
                loader: Some(format!("Fabric {}", stack.loader.version)),
                note: if beta.is_empty() {
                    "Stabled client supported and tested".into()
                } else {
                    format!("Stabled client supported and tested · {} is a beta build for this version", beta.join(", "))
                },
                has_beta_components: !beta.is_empty(),
            }
        }
        None => VersionOption {
            id: id.to_string(),
            release_time,
            supported: false,
            loader: None,
            note: "Not supported by the Stabled client yet".into(),
            has_beta_components: false,
        },
    }
}

pub async fn version_url(id: &str) -> AppResult<(String, String)> {
    let m = manifest().await?;
    for v in m["versions"].as_array().into_iter().flatten() {
        if v["id"].as_str() == Some(id) {
            return Ok((v["url"].as_str().unwrap_or_default().to_string(), v["sha1"].as_str().unwrap_or_default().to_string()));
        }
    }
    crate::bail!("VERSION", "Minecraft {id} was not found in Mojang's version list")
}
