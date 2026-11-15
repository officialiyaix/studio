//! The Store: mods, resource packs and shader packs from Modrinth, installed into one Save.
//!
//! Search and project pages come from Modrinth's public API. Installing picks the newest version of the project
//! made for the Save's Minecraft version (and Fabric, for mods), downloads it from Modrinth's CDN with its SHA-512
//! checked, and adds required dependencies the same way. Files already in the Save are never overwritten.
//! Shader packs need Iris, which is added with the first shader pack if the Save doesn't have it.

use crate::bail;
use crate::error::{AppError, AppResult};
use crate::saves::SaveMeta;
use crate::util::download::{self, Download, Hash};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

const API: &str = "https://api.modrinth.com/v2";
const IRIS: &str = "YL57xq9U";
const LOADERS: &[&str] = &["fabric", "forge", "neoforge", "quilt", "babric", "rift", "liteloader", "modloader", "legacy-fabric", "ornithe", "nilloader", "java-agent", "bukkit", "spigot", "paper", "purpur", "folia", "sponge", "bungeecord", "velocity", "waterfall", "iris", "optifine", "canvas", "vanilla", "minecraft", "datapack"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Mod,
    ResourcePack,
    Shader,
}

impl Kind {
    pub fn parse(s: &str) -> AppResult<Kind> {
        Ok(match s {
            "mod" => Kind::Mod,
            "resourcepack" => Kind::ResourcePack,
            "shader" => Kind::Shader,
            _ => bail!("STORE", "Unknown category"),
        })
    }
    fn folder(self) -> &'static str {
        match self {
            Kind::Mod => "mods",
            Kind::ResourcePack => "resourcepacks",
            Kind::Shader => "shaderpacks",
        }
    }
    fn project_type(self) -> &'static str {
        match self {
            Kind::Mod => "mod",
            Kind::ResourcePack => "resourcepack",
            Kind::Shader => "shader",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub image_url: Option<String>,
    pub color: Option<u32>,
    pub downloads: u64,
    pub follows: u64,
    pub categories: Vec<String>,
    pub date_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub hits: Vec<Hit>,
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub icon_url: Option<String>,
    pub color: Option<u32>,
    pub gallery: Vec<GalleryImage>,
    pub downloads: u64,
    pub follows: u64,
    pub categories: Vec<String>,
    pub license: Option<String>,
    pub source_url: Option<String>,
    pub issues_url: Option<String>,
    pub wiki_url: Option<String>,
    pub discord_url: Option<String>,
    pub published: Option<String>,
    pub updated: Option<String>,
    pub project_type: String,
    pub page_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryImage {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub installed: Vec<String>,
    pub already: Vec<String>,
    pub message: String,
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

fn opt(v: &Value) -> Option<String> {
    v.as_str().filter(|x| !x.is_empty()).map(str::to_owned)
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().into_iter().flatten().filter_map(|x| x.as_str().map(str::to_owned)).collect()
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub async fn search(kind: Kind, query: &str, sort: &str, category: Option<&str>, minecraft: Option<&str>, offset: u64) -> AppResult<SearchResult> {
    let index = match sort {
        "downloads" | "follows" | "newest" | "updated" | "relevance" => sort,
        _ => "relevance",
    };
    let mut facets: Vec<Vec<String>> = vec![vec![format!("project_type:{}", kind.project_type())]];
    if kind == Kind::Mod {
        facets.push(vec!["categories:fabric".into()]);
    }
    if kind == Kind::Shader {
        facets.push(vec!["categories:iris".into()]);
    }
    if let Some(v) = minecraft.filter(|v| crate::minecraft::stack::is_supported(v)) {
        facets.push(vec![format!("versions:{v}")]);
    }
    if let Some(c) = category.filter(|c| valid_id(c)) {
        facets.push(vec![format!("categories:{c}")]);
    }
    let query: String = query.chars().filter(|c| !c.is_control()).take(100).collect();
    let limit = 24u64;
    let resp = download::client()
        .get(format!("{API}/search"))
        .query(&[
            ("query", query.as_str()),
            ("facets", &serde_json::to_string(&facets).unwrap_or_default()),
            ("index", index),
            ("offset", &offset.min(10_000).to_string()),
            ("limit", &limit.to_string()),
        ])
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| AppError::new("STORE_OFFLINE", format!("Couldn't reach Modrinth: {e}")))?;
    if !resp.status().is_success() {
        bail!("STORE", "Modrinth returned {}", resp.status());
    }
    let v: Value = resp.json().await?;
    let hits = v["hits"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|h| Hit {
            project_id: s(&h["project_id"]),
            slug: s(&h["slug"]),
            title: s(&h["title"]),
            description: s(&h["description"]),
            author: s(&h["author"]),
            icon_url: opt(&h["icon_url"]),
            image_url: opt(&h["featured_gallery"]).or_else(|| h["gallery"].as_array().and_then(|g| g.first()).and_then(|x| x.as_str().map(str::to_owned))),
            color: h["color"].as_u64().map(|c| c as u32),
            downloads: h["downloads"].as_u64().unwrap_or(0),
            follows: h["follows"].as_u64().unwrap_or(0),
            // Loader names aren't categories for a Stabled Save.
            categories: strings(&h["display_categories"]).into_iter().filter(|c| !LOADERS.contains(&c.as_str())).collect(),
            date_modified: opt(&h["date_modified"]),
        })
        .collect();
    Ok(SearchResult { hits, total: v["total_hits"].as_u64().unwrap_or(0), offset: v["offset"].as_u64().unwrap_or(offset), limit })
}

pub async fn project(id: &str) -> AppResult<Project> {
    if !valid_id(id) {
        bail!("STORE", "Unknown project");
    }
    let v = download::get_json(&format!("{API}/project/{id}")).await?;
    let gallery = v["gallery"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|g| Some(GalleryImage { url: opt(&g["url"])?, title: opt(&g["title"]) }))
        .collect();
    let project_type = s(&v["project_type"]);
    Ok(Project {
        id: s(&v["id"]),
        slug: s(&v["slug"]),
        title: s(&v["title"]),
        description: s(&v["description"]),
        body: v["body"].as_str().unwrap_or_default().chars().take(60_000).collect(),
        icon_url: opt(&v["icon_url"]),
        color: v["color"].as_u64().map(|c| c as u32),
        gallery,
        downloads: v["downloads"].as_u64().unwrap_or(0),
        follows: v["followers"].as_u64().unwrap_or(0),
        categories: strings(&v["categories"]),
        license: opt(&v["license"]["id"]),
        source_url: opt(&v["source_url"]),
        issues_url: opt(&v["issues_url"]),
        wiki_url: opt(&v["wiki_url"]),
        discord_url: opt(&v["discord_url"]),
        published: opt(&v["published"]),
        updated: opt(&v["updated"]),
        page_url: format!("https://modrinth.com/{}/{}", if project_type.is_empty() { "project".into() } else { project_type.clone() }, s(&v["slug"])),
        project_type,
    })
}

/// The newest version of a project for this Save (release preferred), or None.
async fn best_version(project_id: &str, kind: Kind, minecraft: &str) -> AppResult<Option<Value>> {
    let mut url = format!("{API}/project/{project_id}/version?game_versions={}", urlencode(&json!([minecraft]).to_string()));
    if kind == Kind::Mod {
        url.push_str(&format!("&loaders={}", urlencode(&json!(["fabric"]).to_string())));
    }
    let v = download::get_json(&url).await?;
    let list: Vec<Value> = v.as_array().cloned().unwrap_or_default();
    let release = list.iter().find(|x| x["version_type"] == "release").cloned();
    Ok(release.or_else(|| list.first().cloned()))
}

fn urlencode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

/// Modrinth project ids already present in a Save folder, found by the files' SHA-1 hashes.
pub async fn installed(meta: &SaveMeta, kind: Kind) -> AppResult<HashSet<String>> {
    let dir = meta.game_dir().join(kind.folder());
    let files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().map(|e| e == "jar" || e == "zip" || e == "disabled").unwrap_or(false))
        .collect();
    if files.is_empty() {
        return Ok(HashSet::new());
    }
    let hashes: Vec<String> = tauri::async_runtime::spawn_blocking(move || files.iter().filter_map(|p| download::file_sha1(p)).collect())
        .await
        .map_err(|e| AppError::new("STORE", e.to_string()))?;
    let resp = download::client()
        .post(format!("{API}/version_files"))
        .json(&json!({ "hashes": hashes, "algorithm": "sha1" }))
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| AppError::new("STORE_OFFLINE", format!("Couldn't reach Modrinth: {e}")))?;
    if !resp.status().is_success() {
        bail!("STORE", "Modrinth returned {}", resp.status());
    }
    let v: Value = resp.json().await?;
    Ok(v.as_object().into_iter().flatten().filter_map(|(_, ver)| ver["project_id"].as_str().map(str::to_owned)).collect())
}

/// Installs a project (and its required dependencies) into a Save.
pub async fn install(meta: &SaveMeta, kind: Kind, project_id: &str) -> AppResult<InstallResult> {
    if !valid_id(project_id) {
        bail!("STORE", "Unknown project");
    }
    let mc = meta.minecraft_version.clone();
    let mut have = installed(meta, Kind::Mod).await.unwrap_or_default();
    if kind != Kind::Mod {
        have.extend(installed(meta, kind).await.unwrap_or_default());
    }
    let mut installed_titles = Vec::new();
    let mut already = Vec::new();
    // First in, first out: the project itself, then Iris for shader packs, then required mods.
    let mut queue: std::collections::VecDeque<(String, Kind)> = std::collections::VecDeque::from([(project_id.to_string(), kind)]);
    if kind == Kind::Shader {
        queue.push_back((IRIS.to_string(), Kind::Mod));
    }
    let mut seen = HashSet::new();
    let mut first = true;
    while let Some((id, k)) = queue.pop_front() {
        if !seen.insert(id.clone()) || seen.len() > 12 {
            continue;
        }
        let is_target = first;
        first = false;
        if have.contains(&id) {
            if is_target {
                already.push(id.clone());
            }
            continue;
        }
        let Some(version) = best_version(&id, k, &mc).await? else {
            if is_target {
                bail!("STORE_NO_VERSION", "There's no version of this {} for Minecraft {mc}{}.", match k { Kind::Mod => "mod", Kind::ResourcePack => "resource pack", Kind::Shader => "shader pack" }, if k == Kind::Mod { " on Fabric" } else { "" });
            }
            crate::log_warn!("Store: no version of dependency {id} for {mc}");
            continue;
        };
        let file = version["files"].as_array().and_then(|f| f.iter().find(|x| x["primary"] == true).or_else(|| f.first())).cloned();
        let Some(file) = file else { continue };
        let Some(name) = crate::util::fsx::safe_file_name(file["filename"].as_str().unwrap_or_default()) else {
            bail!("STORE", "This download has a file name Windows can't use.");
        };
        let lower = name.to_ascii_lowercase();
        let ok_ext = match k {
            Kind::Mod => lower.ends_with(".jar"),
            _ => lower.ends_with(".zip"),
        };
        if !ok_ext {
            bail!("STORE", "This download isn't a {} file.", if k == Kind::Mod { ".jar" } else { ".zip" });
        }
        let dir = meta.game_dir().join(k.folder());
        std::fs::create_dir_all(&dir)?;
        let target = dir.join(&name);
        if target.exists() || dir.join(format!("{name}.disabled")).exists() {
            already.push(name.clone());
        } else {
            let sha512 = file["hashes"]["sha512"].as_str().unwrap_or_default().to_string();
            if sha512.len() != 128 {
                bail!("STORE", "Modrinth didn't publish a checksum for this file, so Stabled won't install it.");
            }
            download::fetch(&Download { url: s(&file["url"]), path: target.clone(), hash: Hash::Sha512(sha512), size: file["size"].as_u64() }, None).await?;
            let title = project_title(&id).await.unwrap_or_else(|| name.clone());
            installed_titles.push(title);
            have.insert(id.clone());
        }
        for dep in version["dependencies"].as_array().into_iter().flatten() {
            if dep["dependency_type"] == "required" {
                if let Some(pid) = dep["project_id"].as_str().filter(|p| valid_id(p)) {
                    queue.push_back((pid.to_string(), Kind::Mod));
                }
            }
        }
    }
    let message = match (installed_titles.len(), already.is_empty()) {
        (0, false) => "Already installed in this Save.".to_string(),
        (0, true) => "Nothing was installed.".to_string(),
        (1, _) => format!("Installed {}.", installed_titles[0]),
        (n, _) => format!("Installed {} and {} more ({}).", installed_titles[0], n - 1, installed_titles[1..].join(", ")),
    };
    Ok(InstallResult { installed: installed_titles, already, message })
}

async fn project_title(id: &str) -> Option<String> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<HashMap<String, String>>> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(t) = cache.lock().unwrap().get(id) {
        return Some(t.clone());
    }
    let v = download::get_json(&format!("{API}/project/{id}")).await.ok()?;
    let t = v["title"].as_str()?.to_string();
    cache.lock().unwrap().insert(id.to_string(), t.clone());
    Some(t)
}
