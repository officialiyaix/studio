//! External configuration values (docs/CONFIGURATION.md).
//!
//! Values are baked in at build time from `config/stabled.config.json` or same-named environment variables, and can
//! be overridden at runtime by `%APPDATA%\Stabled\stabled.config.json`. A missing value disables only the feature
//! that needs it, and the UI reports it as "not configured" — never as working.

use serde::Serialize;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StabledConfig {
    #[serde(skip)]
    pub microsoft_client_id: Option<String>,
    pub discord_application_id: Option<String>,
    pub discord_invite_url: Option<String>,
    pub news_feed_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigStatus {
    pub microsoft_configured: bool,
    pub discord_presence_configured: bool,
    pub discord_invite_url: Option<String>,
    pub news_feed_configured: bool,
}

static CONFIG: OnceLock<StabledConfig> = OnceLock::new();

fn non_empty(v: &str) -> Option<String> {
    let t = v.trim();
    (!t.is_empty()).then(|| t.to_string())
}

fn https_url(v: Option<String>) -> Option<String> {
    v.filter(|u| url::Url::parse(u).map(|p| p.scheme() == "https").unwrap_or(false))
}

pub fn get() -> &'static StabledConfig {
    CONFIG.get_or_init(|| {
        let mut cfg = StabledConfig {
            microsoft_client_id: non_empty(env!("STABLED_CFG_MICROSOFT_CLIENT_ID")),
            discord_application_id: non_empty(env!("STABLED_CFG_DISCORD_APPLICATION_ID")),
            discord_invite_url: non_empty(env!("STABLED_CFG_DISCORD_INVITE_URL")),
            news_feed_url: non_empty(env!("STABLED_CFG_NEWS_FEED_URL")),
        };
        let override_path = crate::paths::root().join("stabled.config.json");
        if let Ok(text) = std::fs::read_to_string(&override_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                let take = |key: &str| v.get(key).and_then(|x| x.as_str()).and_then(non_empty);
                if let Some(x) = take("MICROSOFT_CLIENT_ID") {
                    cfg.microsoft_client_id = Some(x);
                }
                if let Some(x) = take("DISCORD_APPLICATION_ID") {
                    cfg.discord_application_id = Some(x);
                }
                if let Some(x) = take("DISCORD_INVITE_URL") {
                    cfg.discord_invite_url = Some(x);
                }
                if let Some(x) = take("NEWS_FEED_URL") {
                    cfg.news_feed_url = Some(x);
                }
            }
        }
        // Validate shapes so a malformed value is treated as not configured rather than failing later.
        cfg.microsoft_client_id = cfg.microsoft_client_id.filter(|id| uuid::Uuid::parse_str(id).is_ok());
        cfg.discord_application_id = cfg
            .discord_application_id
            .filter(|id| id.len() >= 17 && id.len() <= 20 && id.chars().all(|c| c.is_ascii_digit()));
        cfg.discord_invite_url = https_url(cfg.discord_invite_url).filter(|u| {
            url::Url::parse(u)
                .ok()
                .and_then(|p| p.host_str().map(|h| h == "discord.gg" || h == "discord.com" || h.ends_with(".discord.com")))
                .unwrap_or(false)
        });
        cfg.news_feed_url = https_url(cfg.news_feed_url);
        cfg
    })
}

pub fn status() -> ConfigStatus {
    let c = get();
    ConfigStatus {
        microsoft_configured: c.microsoft_client_id.is_some(),
        discord_presence_configured: c.discord_application_id.is_some(),
        discord_invite_url: c.discord_invite_url.clone(),
        news_feed_configured: c.news_feed_url.is_some(),
    }
}
