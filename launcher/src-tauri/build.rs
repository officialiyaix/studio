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

    // The Stabled Client builds (one per Minecraft version) are bundled from resources/clients/.
    // Build them with `npm run bundle-client`, which runs Gradle in /client and copies the jars here.
    let clients = Path::new("resources/clients");
    let count = std::fs::read_dir(clients).map(|d| d.filter_map(Result::ok).filter(|e| e.path().extension().map(|x| x == "jar").unwrap_or(false)).count()).unwrap_or(0);
    if count == 0 {
        let _ = std::fs::create_dir_all(clients);
        // Tauri requires the resource glob to match something; a README keeps the folder present.
        let _ = std::fs::write(clients.join("README.txt"), "Stabled Client builds are copied here by `npm run bundle-client`.\n");
        println!("cargo:warning=No Stabled client builds in resources/clients: run `npm run bundle-client` before packaging. Saves can't be launched without them.");
    }
    println!("cargo:rerun-if-changed=resources/clients");
    tauri_build::build()
}
