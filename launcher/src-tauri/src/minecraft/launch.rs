//! Builds and starts the game process for a Save. Authentication has already been enforced by the caller
//! (`commands::save_launch`), which refuses to reach this code without a valid Minecraft session and entitlement.

use super::install::Prepared;
use super::rules;
use crate::bail;
use crate::error::AppResult;
use crate::paths;
use crate::saves::SaveMeta;
use crate::util::log;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct LaunchAuth {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub xuid: String,
    pub client_id: String,
}

pub fn build(
    save: &SaveMeta,
    prepared: &Prepared,
    auth: &LaunchAuth,
    memory_mb: u32,
    extra_jvm: &[String],
) -> AppResult<(PathBuf, Vec<String>)> {
    let sep = if cfg!(windows) { ";" } else { ":" };
    for p in &prepared.classpath {
        if !p.is_file() {
            bail!("INSTALL", "A game file is missing ({}). Use Repair in this Save's menu.", p.file_name().unwrap_or_default().to_string_lossy());
        }
    }
    let classpath = prepared.classpath.iter().map(|p| p.to_string_lossy().into_owned()).collect::<Vec<_>>().join(sep);
    let profile_id = prepared.fabric_profile["id"].as_str().unwrap_or_default().to_string();
    let natives = paths::natives_dir().join(&save.minecraft_version);
    std::fs::create_dir_all(&natives)?;
    let vars: HashMap<&str, String> = HashMap::from([
        ("auth_player_name", auth.username.clone()),
        ("version_name", profile_id.clone()),
        ("game_directory", save.game_dir().to_string_lossy().into_owned()),
        ("assets_root", paths::assets_dir().to_string_lossy().into_owned()),
        ("assets_index_name", prepared.asset_index.clone()),
        ("auth_uuid", auth.uuid.replace('-', "")),
        ("auth_access_token", auth.access_token.clone()),
        ("clientid", auth.client_id.clone()),
        ("auth_xuid", auth.xuid.clone()),
        ("user_type", "msa".into()),
        ("version_type", "release".into()),
        ("natives_directory", natives.to_string_lossy().into_owned()),
        ("launcher_name", "Stabled".into()),
        ("launcher_version", env!("CARGO_PKG_VERSION").into()),
        ("classpath", classpath.clone()),
        ("classpath_separator", sep.into()),
        ("user_properties", "{}".into()),
        ("library_directory", paths::libraries_dir().to_string_lossy().into_owned()),
    ]);
    let substitute = |s: &str| -> String {
        let mut out = s.to_string();
        for (k, v) in &vars {
            out = out.replace(&format!("${{{k}}}"), v);
        }
        out
    };
    let features: [(&str, bool); 2] = [("is_demo_user", false), ("has_custom_resolution", false)];
    let mut args: Vec<String> = Vec::new();
    args.push(format!("-Xmx{memory_mb}M"));
    args.push(format!("-Xms{}M", memory_mb.min(1024)));
    // Before 1.13 (1.8.9, 1.9.4) the version file has a single `minecraftArguments` string and no JVM arguments, so
    // the launcher supplies the natives path and class path itself.
    let legacy_game_args = prepared.version_json["minecraftArguments"].as_str();
    if legacy_game_args.is_some() {
        args.push(format!("-Djava.library.path={}", natives.display()));
        args.push("-Dminecraft.launcher.brand=Stabled".into());
        args.push(format!("-Dminecraft.launcher.version={}", env!("CARGO_PKG_VERSION")));
        args.push("-cp".into());
        args.push(classpath);
    }
    args.extend(rules::expand_arguments(prepared.version_json["arguments"].get("jvm"), &features).iter().map(|a| substitute(a)));
    args.extend(rules::expand_arguments(prepared.fabric_profile["arguments"].get("jvm"), &features).iter().map(|a| substitute(a)));
    if let Some(l) = &prepared.logging_arg {
        args.push(l.clone());
    }
    // Contract with the in-game client (docs/CONTRACTS.md).
    args.push(format!("-Dstabled.home={}", paths::root().display()));
    args.push(format!("-Dstabled.saveDir={}", save.root().display()));
    if let Some(dir) = crate::gallery::dir() {
        args.push(format!("-Dstabled.galleryDir={}", dir.display()));
    }
    // The Save's game mode, shown on the in-game title screen.
    let mode = save.effective_game_mode();
    if let Some(g) = crate::minecraft::stack::game_modes().iter().find(|g| g.id == mode) {
        if !stack_mode_is_empty(save, &mode) {
            args.push(format!("-Dstabled.gameMode={}", g.name));
        }
    }
    // Streamer mode: the in-game title screen hides the player's name too.
    let streamer = crate::settings::Settings::load().streamer;
    if streamer.enabled && streamer.hide_username {
        args.push("-Dstabled.streamer.hideName=true".into());
    }
    if let Some(id) = &crate::config::get().discord_application_id {
        args.push(format!("-Dstabled.discordAppId={id}"));
    }
    // PC Optimiser → Tuned Java memory management, unless the player picked a collector themselves.
    let own_gc = extra_jvm.iter().any(|a| a.starts_with("-XX:+Use") && a.ends_with("GC"));
    if crate::optimiser::launch_tweaks().gc_flags && !own_gc {
        args.extend(crate::optimiser::GC_FLAGS.iter().map(|f| f.to_string()));
    }
    args.extend(extra_jvm.iter().cloned());
    let main_class = prepared.fabric_profile["mainClass"].as_str().unwrap_or("net.fabricmc.loader.impl.launch.knot.KnotClient");
    args.push(main_class.to_string());
    match legacy_game_args {
        Some(line) => args.extend(line.split_whitespace().map(|a| substitute(a))),
        None => args.extend(rules::expand_arguments(prepared.version_json["arguments"].get("game"), &features).iter().map(|a| substitute(a))),
    }
    args.extend(rules::expand_arguments(prepared.fabric_profile["arguments"].get("game"), &features).iter().map(|a| substitute(a)));
    Ok((prepared.java.clone(), args))
}

/// Whether a game mode adds nothing for this Save's version (e.g. Standard), so there's nothing to show.
fn stack_mode_is_empty(save: &SaveMeta, mode: &str) -> bool {
    crate::minecraft::stack::for_version(&save.minecraft_version).map(|st| crate::minecraft::stack::mode_stack(st, mode).mods.is_empty()).unwrap_or(true)
}

pub struct Started {
    pub child: tokio::process::Child,
    pub pid: u32,
}

pub async fn spawn(save: &SaveMeta, java: PathBuf, args: Vec<String>, secret: &str) -> AppResult<Started> {
    log::register_secret(secret);
    let mut cmd = tokio::process::Command::new(&java);
    cmd.args(&args).current_dir(save.game_dir()).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().map_err(|e| crate::error::AppError::new("LAUNCH", format!("Could not start Java: {e}")))?;
    let pid = child.id().unwrap_or(0);
    // PC Optimiser → High priority for Minecraft.
    #[cfg(windows)]
    if crate::optimiser::launch_tweaks().high_priority {
        crate::optimiser::boost_process(&child);
    }
    crate::log_info!("Started Save {} as process {pid}", save.id);
    let log_path = save.game_dir().join("logs").join("stabled-launch.log");
    let _ = std::fs::create_dir_all(save.game_dir().join("logs"));
    let out = child.stdout.take();
    let err = child.stderr.take();
    tokio::spawn(async move {
        let Ok(mut file) = tokio::fs::File::create(&log_path).await else { return };
        let header = format!("Stabled {} launch log — {}\n", env!("CARGO_PKG_VERSION"), chrono::Local::now().to_rfc3339());
        let _ = file.write_all(header.as_bytes()).await;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        if let Some(o) = out {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(o).lines();
                while let Ok(Some(l)) = lines.next_line().await {
                    let _ = tx.send(l);
                }
            });
        }
        if let Some(e) = err {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(e).lines();
                while let Ok(Some(l)) = lines.next_line().await {
                    let _ = tx.send(l);
                }
            });
        }
        drop(tx);
        while let Some(line) = rx.recv().await {
            let _ = file.write_all(log::redact(&line).as_bytes()).await;
            let _ = file.write_all(b"\n").await;
        }
    });
    Ok(Started { child, pid })
}
