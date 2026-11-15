//! Developer tooling, compiled only with `--features dev-tools`. It exercises the real install pipeline and
//! launch-argument builder without the UI. Launching uses an offline development profile and placeholder token, so
//! it can only reach singleplayer; the shipped launcher never launches without a verified Microsoft session.

use crate::minecraft::{install, launch, stack};
use crate::saves;

pub fn main(args: &[String]) -> i32 {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        match args.first().map(String::as_str) {
            Some("create") => {
                let name = args.get(1).cloned().unwrap_or_else(|| "Dev Save".into());
                let version = args.get(2).cloned().unwrap_or_else(|| stack::PRIMARY_VERSION.into());
                let game_mode = args.get(3).cloned();
                let req = saves::CreateRequest { name, minecraft_version: version, icon_png_base64: None, memory_mb: Some(2048), performance_preset: None, game_mode };
                match saves::create(req, &crate::settings::Settings::load()) {
                    Ok(m) => { println!("{}", m.id); 0 }
                    Err(e) => { eprintln!("{e}"); 1 }
                }
            }
            Some("install") | Some("command") => {
                let id = args.get(1).cloned().unwrap_or_default();
                saves::managed::set_bundled_clients_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources").join("clients"));
                crate::gallery::set_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources").join("backgrounds"));
                let meta = match saves::load(&id) { Ok(m) => m, Err(e) => { eprintln!("{e}"); return 1; } };
                let emit = |p: install::Progress| eprintln!("[{}] {} {}/{} files {}/{} bytes", p.phase, p.label, p.files_done, p.files_total, p.bytes_done, p.bytes_total);
                let prepared = match install::prepare(&meta, 8, false, &emit).await { Ok(p) => p, Err(e) => { eprintln!("install failed: {e}"); return 1; } };
                let mut m = saves::load(&id).unwrap();
                m.installed_stack = stack::for_version(&m.minecraft_version).map(|s| s.key());
                let _ = m.save();
                if args[0] == "command" {
                    let auth = launch::LaunchAuth { username: "StabledDev".into(), uuid: "00000000-0000-0000-0000-000000000000".into(), access_token: "0".into(), xuid: "0".into(), client_id: String::new() };
                    let (java, a) = launch::build(&meta, &prepared, &auth, 2048, &[]).unwrap();
                    println!("{}", serde_json::to_string(&serde_json::json!({ "java": java, "args": a, "cwd": meta.game_dir() })).unwrap());
                } else {
                    eprintln!("installed");
                }
                0
            }
            _ => { eprintln!("usage: --dev create <name> [minecraft version] [game mode] | install <id> | command <id>"); 2 }
        }
    })
}
