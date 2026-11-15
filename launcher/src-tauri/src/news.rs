//! News panel data.
//!
//! Sources, in order:
//! 1. The Stabled feed at `NEWS_FEED_URL` (format: docs/NEWS_FORMAT.md) — only when configured.
//! 2. Minecraft: Java Edition release notes from Mojang's launcher content feed (real titles and dates), when
//!    "Show Minecraft release notes" is on.
//! 3. Built-in Stabled guides: product help written for this launcher, labelled "Guide" and never dated.
//!
//! Remote results are cached; when fetching fails, the cache is shown and labelled with when it was fetched.

use crate::error::AppResult;
use crate::util::{download::client, fsx};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const MOJANG_BASE: &str = "https://launchercontent.mojang.com";
const MOJANG_PATCH_NOTES: &str = "https://launchercontent.mojang.com/v2/javaPatchNotes.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    pub id: String,
    /// "stabled" | "minecraft" | "guide"
    pub source: String,
    pub title: String,
    pub summary: String,
    pub category: String,
    pub date: Option<String>,
    pub image: Option<String>,
    pub url: Option<String>,
    pub body: Option<String>,
    /// For Minecraft release notes: where the full article body is fetched from.
    #[serde(default)]
    pub content_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsResult {
    pub items: Vec<NewsItem>,
    /// "live" | "cached" | "local" | "failed"
    pub state: String,
    pub fetched_at: Option<String>,
    pub error: Option<String>,
    pub feed_configured: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cache {
    fetched_at: String,
    items: Vec<NewsItem>,
}

fn cache_path() -> std::path::PathBuf {
    crate::paths::cache_dir().join("news.json")
}

fn https(u: &str) -> Option<String> {
    url::Url::parse(u).ok().filter(|p| p.scheme() == "https").map(|p| p.to_string())
}

fn clamp_text(s: &str, n: usize) -> String {
    let t: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > n {
        format!("{}…", t.chars().take(n).collect::<String>().trim_end())
    } else {
        t
    }
}

async fn fetch_json(url: &str) -> AppResult<Value> {
    let resp = client().get(url).timeout(Duration::from_secs(12)).send().await?;
    if !resp.status().is_success() {
        crate::bail!("NETWORK", "The news source returned {}", resp.status());
    }
    Ok(resp.json().await?)
}

async fn stabled_feed(url: &str) -> AppResult<Vec<NewsItem>> {
    let v = fetch_json(url).await?;
    let mut out = Vec::new();
    for (i, it) in v["items"].as_array().into_iter().flatten().enumerate().take(50) {
        let Some(title) = it["title"].as_str().filter(|t| !t.trim().is_empty()) else { continue };
        let date = it["date"].as_str().filter(|d| chrono::DateTime::parse_from_rfc3339(d).is_ok() || chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok()).map(str::to_owned);
        out.push(NewsItem {
            id: format!("stabled:{}", it["id"].as_str().map(str::to_owned).unwrap_or_else(|| i.to_string())),
            source: "stabled".into(),
            title: clamp_text(title, 120),
            summary: clamp_text(it["summary"].as_str().unwrap_or_default(), 220),
            category: clamp_text(it["category"].as_str().unwrap_or("Stabled"), 24),
            date,
            image: it["image"].as_str().and_then(https),
            url: it["url"].as_str().and_then(https),
            body: it["body"].as_str().map(|b| b.chars().take(20_000).collect()),
            content_path: None,
        });
    }
    Ok(out)
}

async fn minecraft_notes() -> AppResult<Vec<NewsItem>> {
    let v = fetch_json(MOJANG_PATCH_NOTES).await?;
    let mut out = Vec::new();
    for e in v["entries"].as_array().into_iter().flatten() {
        if e["type"].as_str() != Some("release") {
            continue;
        }
        let Some(title) = e["title"].as_str() else { continue };
        let image = e["image"]["url"].as_str().map(|p| if p.starts_with("http") { p.to_string() } else { format!("{MOJANG_BASE}{p}") }).and_then(|u| https(&u));
        out.push(NewsItem {
            id: format!("minecraft:{}", e["id"].as_str().or(e["version"].as_str()).unwrap_or(title)),
            source: "minecraft".into(),
            title: title.to_string(),
            summary: clamp_text(e["shortText"].as_str().unwrap_or_default(), 220),
            category: "Minecraft".into(),
            date: e["date"].as_str().map(str::to_owned),
            image,
            url: None,
            body: None,
            content_path: e["contentPath"].as_str().map(str::to_owned),
        });
        if out.len() >= 8 {
            break;
        }
    }
    Ok(out)
}

pub async fn get(show_minecraft: bool, force_cache_only: bool) -> NewsResult {
    let feed = crate::config::get().news_feed_url.clone();
    let mut items = Vec::new();
    let mut errors = Vec::new();
    let mut remote_attempted = false;
    if !force_cache_only {
        if let Some(url) = &feed {
            remote_attempted = true;
            match stabled_feed(url).await {
                Ok(mut v) => items.append(&mut v),
                Err(e) => errors.push(format!("Stabled news: {e}")),
            }
        }
        if show_minecraft {
            remote_attempted = true;
            match minecraft_notes().await {
                Ok(mut v) => items.append(&mut v),
                Err(e) => errors.push(format!("Minecraft release notes: {e}")),
            }
        }
    }
    let mut result = NewsResult { items: vec![], state: "local".into(), fetched_at: None, error: None, feed_configured: feed.is_some() };
    if !items.is_empty() {
        items.sort_by(|a, b| b.date.cmp(&a.date));
        let now = chrono::Utc::now().to_rfc3339();
        let _ = fsx::write_json(&cache_path(), &Cache { fetched_at: now.clone(), items: items.clone() });
        result.state = if errors.is_empty() { "live" } else { "cached" }.into();
        result.fetched_at = Some(now);
        result.items = items;
        result.error = (!errors.is_empty()).then(|| errors.join(" · "));
    } else if remote_attempted || force_cache_only {
        if let Ok(cache) = fsx::read_json::<Cache>(&cache_path()) {
            result.state = "cached".into();
            result.fetched_at = Some(cache.fetched_at);
            result.items = cache.items.into_iter().filter(|i| show_minecraft || i.source != "minecraft").collect();
        } else {
            result.state = if remote_attempted { "failed" } else { "local" }.into();
        }
        result.error = (!errors.is_empty()).then(|| errors.join(" · "));
    }
    result.items.extend(guides());
    result
}

/// Full text for the reader view. Minecraft notes are fetched from Mojang; others use their own body.
pub async fn detail(item: &NewsItem) -> AppResult<String> {
    if let Some(path) = &item.content_path {
        if !path.starts_with('/') || path.contains("..") {
            crate::bail!("NEWS", "Invalid article path");
        }
        let v = fetch_json(&format!("{MOJANG_BASE}{path}")).await?;
        return Ok(v["body"].as_str().unwrap_or_default().to_string());
    }
    Ok(item.body.clone().unwrap_or_else(|| item.summary.clone()))
}

fn guide(id: &str, title: &str, summary: &str, body: &str) -> NewsItem {
    NewsItem {
        id: format!("guide:{id}"),
        source: "guide".into(),
        title: title.into(),
        summary: summary.into(),
        category: "Guide".into(),
        date: None,
        image: None,
        url: None,
        body: Some(body.into()),
        content_path: None,
    }
}

pub fn guides() -> Vec<NewsItem> {
    vec![
        guide(
            "saves",
            "What is a Save?",
            "A Save is a complete, separate Minecraft installation — not a single world.",
            "A Save is its own Minecraft installation with its own mods, settings, worlds, resource packs, shader packs, screenshots and logs.\n\nChanging mods or options in one Save never affects another, so you can keep a lightweight Save for multiplayer and a heavily modded one for singleplayer side by side.\n\nGame files that never change (Minecraft itself, libraries, sounds and the Java runtime) are downloaded once and shared, so extra Saves cost little disk space.",
        ),
        guide(
            "performance",
            "Getting the best performance",
            "Presets, memory and the PC Optimiser — where each one helps.",
            "Every Save includes Sodium, Lithium, FerriteCore, ImmediatelyFast and EntityCulling, pinned to versions tested with the Stabled client.\n\nIn game, open Stabled Settings → Performance to pick Performance, Balanced or Quality. Presets apply immediately; only switching between OpenGL and Vulkan needs a restart.\n\nMost Saves run well with 4 GB of memory. Raise it in the Save's settings only for large modpacks — giving Java more than it needs doesn't make the game faster.\n\nThe PC Optimiser page lists Windows settings that commonly affect games. Nothing changes until you choose Apply, and every change can be restored.",
        ),
        guide(
            "mods",
            "Adding your own mods",
            "Drop in Fabric mods per Save, with conflict checks.",
            "Open a Save and choose Mods → Add mods to copy .jar files into that Save only. Stabled reads each mod's Fabric metadata and points out problems it can detect: Forge or Quilt mods, the wrong Minecraft version, missing dependencies, duplicates and declared incompatibilities. Not every problem can be predicted, so check a mod's page if the game won't start.\n\nComponents marked \"Stabled\" are managed for you. The Stabled client and Fabric API are required; the performance mods can be disabled. If you add your own copy of a managed mod, Stabled uses yours and tells you it skipped its own.\n\nWhile a Save is running its mods are locked, so files can't change underneath the game.",
        ),
        guide(
            "discord",
            "Discord Presence",
            "What Stabled shows on your Discord profile, and how to turn it off.",
            "Discord Presence is on for new Saves. When Discord is running on this PC, your profile shows Stabled Client with general activity — in menus, playing singleplayer, playing multiplayer or Realms — and how long you've been playing.\n\nServer addresses, world names and account details are never shared, and Stabled never needs your Discord password or token. Turn it off per Save in the Save's settings or in game under Stabled Settings → Discord Presence; the activity clears straight away.",
        ),
        guide(
            "appearance",
            "Themes and Frosted Glass",
            "Nine themes, two surface styles, one look in the launcher and in game.",
            "Settings → Appearance offers eight themes plus Custom Background, which uses an image from your computer. Stabled keeps its own copy, so moving or deleting the original is fine.\n\nUnder the themes, choose Simple for solid surfaces with minimal effects, or Frosted Glass for translucent panels with a soft blur. With Frosted Glass, the Transparency slider sets how much of the background shows through: 10% is nearly solid, 90% is nearly clear. Text stays fully opaque either way.\n\nTheme and surface style are shared by the launcher and every Save's in-game menus.",
        ),
    ]
}
