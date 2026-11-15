# Architecture

Stabled has two deliverables that share a small set of files:

```
┌──────────────────────── Stabled launcher (Tauri 2) ────────────────────────┐
│ React UI (launcher/src)            Rust backend (launcher/src-tauri/src)   │
│  shell: rail · header · specs      commands.rs  — validated command API   │
│  Home · Saves · PC Optimiser ·     saves/       — Save metadata, managed   │
│  Settings · Create Save ·          mods.rs      — mods + conflict checks   │
│  sign-in popup                     minecraft/   — manifest, install,       │
│                                                   Java runtime, launch     │
│                                    auth/        — Microsoft → Minecraft    │
│                                    system.rs    — CPU/GPU/RAM/OS           │
│                                    news.rs · optimiser/ · settings.rs      │
└──────────────┬──────────────────────────────────────────────┬─────────────┘
               │ starts javaw with -Dstabled.* properties      │ reads/writes
               ▼                                               ▼
┌──────── Minecraft 26.3 + Fabric ────────┐      %APPDATA%\Stabled\appearance.json
│ Stabled Client mod (client/)            │      saves\<id>\stabled\*.json
│  menus · settings · intro · FPS HUD ·   │      (see CONTRACTS.md)
│  motion blur · Discord IPC              │
└─────────────────────────────────────────┘
```

## Why these choices

- **Tauri 2 + Rust** keeps the launcher small (no bundled Chromium; WebView2 on Windows) and gives direct, typed access to Windows APIs: DXGI for GPUs, DPAPI for credentials, the registry and `ShellExecuteEx` for the optimiser. The UI cannot touch the file system or network directly; every action goes through a validated command, and no command returns a token.
- **Fabric** is the loader with first-class support for Minecraft 26.3 on release day, the official 26.3 toolchain (non-remapping Loom 1.19, Java 25), and the strongest client performance ecosystem (Sodium, Lithium, …). Versions were verified against `meta.fabricmc.net`, the official `fabric-example-mod` 26.3 branch and Modrinth.
- **A pinned stack per Minecraft version** (`launcher/src-tauri/stack/26.3.json`) rather than "latest at install time": every Save gets the exact combination that was tested, each file is verified by SHA-512, and nothing changes underneath the user without a Stabled update.

## Data layout

| Location | Contents | Mutable |
| --- | --- | --- |
| `%APPDATA%\Stabled\settings.json` | Launcher settings | yes |
| `%APPDATA%\Stabled\appearance.json` | Theme and surface style (shared with the game) | yes |
| `%APPDATA%\Stabled\backgrounds\custom.png` | Managed copy of the Custom Background | replaced on upload |
| `%APPDATA%\Stabled\secure\account.bin` | Microsoft/Minecraft session, DPAPI-encrypted | yes |
| `%APPDATA%\Stabled\saves\<uuid>\` | One Save (see below) | yes |
| `%APPDATA%\Stabled\optimiser\history.json` | Values the optimiser replaced | yes |
| `%APPDATA%\Stabled\logs\launcher.log` | Launcher log (secrets redacted) | yes |
| `%LOCALAPPDATA%\Stabled\shared\` | Game jars, libraries, assets, Java runtimes, mod downloads | immutable, content-verified |

A Save folder:

```
save.json        name, version, icon, memory, installed stack — the folder name is a UUID, so renaming is metadata-only
icon.png         256 × 256 managed icon (optional)
stabled/         client.json · onboarding.json · managed.json · runtime.json
minecraft/       the game directory: mods, config, saves (worlds), resourcepacks, shaderpacks, screenshots, logs, options.txt
```

Nothing mutable is shared between Saves. Duplicating copies the whole folder; deleting moves it to the Recycle Bin.

## Install and launch

1. `install::prepare` resolves Mojang's version JSON (SHA-1 checked), the Fabric loader profile, libraries (rule-filtered for the platform; LWJGL natives self-extract), the asset index and objects, and Mojang's own Java 25 runtime (`java-runtime-epsilon`). Downloads are parallel, retried, size- and hash-verified, and limited to an allow-list of official hosts. The first install of a stack does a full hash check; later launches check presence and size.
2. `saves::managed::sync` installs the bundled Stabled client jar and the pinned stack into the Save's own `mods` folder, never overwriting a file Stabled did not install (see "Mods" below).
3. `commands::save_launch` asks `AuthManager::launch_credentials` for a valid Minecraft token **before** any download or process. Without a signed-in account that owns Java Edition it returns `SIGN_IN_REQUIRED` / `NO_JAVA_ACCESS`, and the UI opens the sign-in popup. This is enforced in the backend, not only the UI (see `auth::tests::launch_refused_without_session`).
4. `launch::build` expands Mojang's and Fabric's argument templates, adds memory and the Stabled properties, and starts `javaw.exe` without a console window. Output is captured to `minecraft/logs/stabled-launch.log` with the access token redacted.
5. While running, the Save is locked: mods, Save settings, duplicate and delete are refused by the backend. When the game exits the launcher re-reads the shared appearance (the theme may have been changed in game).

## Microsoft sign-in

Authorization code + PKCE in the system browser, redirect to a one-shot loopback listener on `http://localhost:<random port>`, then Xbox Live → XSTS (`rp://api.minecraftservices.com/`) → `login_with_xbox` → entitlement check (`/entitlements/license`, falling back to `/entitlements/mcstore`) → profile. A Microsoft login without Java Edition is reported distinctly from a successful one. XSTS error codes (no Xbox profile, region, child account) are explained. On every startup the stored session is restored and refreshed if needed; expiry signs the user out with a message; network failure keeps the profile but marks the session as needing refresh at launch.

Tokens are registered with the logger's redaction list, never serialised to the UI, never written to `settings.json`, and stored only in `secure/account.bin` via DPAPI (current user, Stabled-specific entropy). On non-Windows development builds the session is kept in memory only.

## Mods

`mods.rs` reads each jar's `fabric.mod.json` (and nested jars) without extracting it and reports: non-Fabric loaders, duplicate mod ids, Minecraft version ranges, missing dependencies, and declared `breaks` whose version range matches what is installed. The UI states that not every conflict can be predicted.

Managed components (`stabled/managed.json`): the Stabled client and Fabric API are required (restored if missing, can't be disabled or removed); performance mods are optional (disable, remove, restore). A managed file is replaced only if it still matches the hash Stabled installed. If the user adds their own copy of a managed mod, Stabled uses theirs and says so.

## In-game client

See [CONTRACTS.md](CONTRACTS.md) for the files and properties it reads. Highlights:

- **Menus** — `TitleScreenMixin` re-lays out the vanilla title screen's own widgets into the Stabled layout (so demo mode, Realms, friends, language, accessibility and other mods' buttons keep working) and replaces the logo and backdrop. `PauseScreenMixin` adds Stabled Settings above Disconnect. `AbstractButtonMixin` themes buttons only on those menus and Stabled's own screens.
- **Introduction** — opened from the client tick when the title screen is showing and the Save's onboarding isn't complete, so it appears after Minecraft's own first-run accessibility prompt and before world or server selection.
- **FPS display** — a Fabric `HudElement` drawing `Minecraft.getFps()` (the renderer's per-second frame count).
- **Motion blur** — `MotionBlurRenderer` runs after the level and vanilla post effects and before `guiRenderer.render()`. It blends the frame with a ping-pong history target through a RenderPearl pipeline and a mappable uniform ring buffer; the weight is `retention^(dt·60)`, so the trail length is independent of frame rate. Targets are reallocated on resize, reseeded on world change, and released when the effect is off. If the pipeline can't compile or an Iris shader pack is active, the control is disabled with an explanation.
- **Discord Presence** — `DiscordIpc` speaks the desktop IPC protocol over `\\.\pipe\discord-ipc-N` (Unix sockets on other platforms). One worker thread per game process, exponential back-off (5 s → 60 s), updates coalesced to one per 2 s, immediate clear on disable, clear and close on exit.

## PC Optimiser

The earlier Windows optimiser's source was not present in this repository, so none of its actions are included or claimed; the page says so. `optimiser/` provides Stabled's own actions, each with detection, explicit Apply, a recorded previous value and Restore. Only Hardware-accelerated GPU scheduling (HKLM) needs administrator rights; it runs in a short-lived elevated copy of Stabled (`--stabled-optimiser-elevated`) that accepts only machine-wide actions. New actions — including the original optimiser's, if its source becomes available — plug into `optimiser::ACTIONS`.
