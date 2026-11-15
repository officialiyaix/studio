use std::path::Path;

/// Configuration keys documented in docs/CONFIGURATION.md.
const KEYS: [&str; 4] = ["MICROSOFT_CLIENT_ID", "DISCORD_APPLICATION_ID", "DISCORD_INVITE_URL", "NEWS_FEED_URL"];

fn main() {
    let config_path = Path::new("../../config/stabled.config.json");
    println!("cargo:rerun-if-changed={}", config_path.display());
    let file: serde_json::Value = std::fs::read_to_string(config_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::Value::Null);
    for key in KEYS {
        println!("cargo:rerun-if-env-changed={key}");
        let value = std::env::var(key)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| file.get(key).and_then(|v| v.as_str()).map(str::to_owned))
            .unwrap_or_default();
        println!("cargo:rustc-env=STABLED_CFG_{key}={}", value.trim());
    }

    // The Stabled Client mod jar is bundled as a resource. Build it first with `./gradlew build` in /client.
    let jar = Path::new("resources/stabled-client.jar");
    if !jar.exists() {
        println!("cargo:warning=resources/stabled-client.jar is missing: run `npm run bundle-client` (builds /client) before packaging. Launching Saves will report the missing component.");
    }
    println!("cargo:rerun-if-changed=resources/stabled-client.jar");
    tauri_build::build()
}
