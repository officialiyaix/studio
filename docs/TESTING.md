# Testing

This records what has actually been exercised, how, and what still needs a Windows PC, real hardware or the external configuration values. Screenshots from these runs are in [`screenshots/`](screenshots/).

## Environment used

A Linux container with Xvfb and Mesa software rendering (llvmpipe for OpenGL 4.5, lavapipe for Vulkan 1.4), JDK 25, Rust 1.97 and Node 22. Network access to Mojang, Fabric and Modrinth; no access to Microsoft account services, Discord or Mojang's launcher-content (news) host.

## Verified

### In-game client (real Minecraft 26.3 + Fabric 0.20.1 + the full pinned stack)

The game was started with the exact command the launcher builds (`--features dev-tools` CLI, offline development profile because no Microsoft client ID is configured).

| Area | Result |
| --- | --- |
| Loading | All 57 mods load (Fabric API modules, Sodium, Lithium, FerriteCore, ImmediatelyFast, EntityCulling, Stabled Client); no mixin or Stabled errors |
| Introduction | Opens after Minecraft's first-run accessibility prompt and before world/server selection; Step 1–5 with progress bar; Back from step 2; Skip immediately beside Next; Finish on step 5; step saved to `onboarding.json` on each move; `completed` + timestamp written on Finish; not shown again on the next start |
| Appearance step | Nine theme tiles; Simple/Frosted Glass below; Transparency slider appears only for glass, updates samples live while dragging (verified at 35% and 75%), persists on release to the shared `appearance.json` |
| Main menu | Stabled layout over the vanilla title screen with all vanilla controls (Singleplayer, Multiplayer, Realms, Options, Quit, friends, language, accessibility, credits) plus Stabled Settings |
| Pause menu | Stabled panel; frosted blur in glass mode; Stabled Settings inserted above Save and Quit |
| Stabled Settings | Sectioned screen; changes persist to `client.json` immediately |
| FPS display | Shows `Minecraft.getFps()` in game (11–51 FPS on software rendering), top-left, panel background; scales with GUI scale |
| Motion blur | Visible trails while turning, HUD/hotbar/crosshair/FPS sharp — on **both OpenGL and Vulkan** backends; window resize reallocates buffers without stale frames or errors |
| Discord Presence | Against a mock server implementing Discord's IPC framing: handshake with the application ID; `SET_ACTIVITY` "In menus" → "Playing singleplayer" on world join; elapsed start time; turning it off sends a clear and closes the connection; turning it on reconnects with the same session start; connection closes when the game process ends |
| Runtime status | `runtime.json` reports Discord status, motion-blur availability and whether the introduction is open |

### Launcher backend

| Area | Result |
| --- | --- |
| Builds | Linux debug build; `cargo check --target x86_64-pc-windows-msvc` passes, and a deliberately injected type error in `optimiser/windows_impl.rs` was caught — so the DPAPI, DXGI, registry and UAC code is compiled against the real `windows` crate APIs |
| Unit tests | Fabric version predicates; version-aware `breaks`; launch refused without a session (`SIGN_IN_REQUIRED`/`AUTH_UNCONFIGURED`); account debug output never contains tokens |
| Install pipeline | Real downloads: Mojang version JSON, client jar, 73 libraries, 5,149 assets, Java 25 runtime, Fabric profile; SHA-1/SHA-512 verified; managed stack copied into the Save's own `mods` folder; second Save installs from the shared cache in seconds with its own copies |
| Mod checks | Correctly reports nothing for the pinned stack; correctly reads Sodium's declared incompatibility with Iris ≤ 1.12.0 |

### Launcher UI (the real Tauri app, WebKitGTK)

Startup sign-in popup over the blurred launcher (with the honest "not configured" state), dismissal, no re-prompt during navigation; Home, Saves (Overview/Mods/Save settings), PC Optimiser, Settings; navigation rail hover expansion over content without layout shift; theme switching (Stabled Night, Cloud, Sunset, Custom Background) and Simple/Frosted Glass applied instantly and persisted; Create Save field order, version list from Mojang's live manifest with supported/unsupported marking, inline validation (empty and duplicate names), creation; Install with live progress → Ready; **Launch while signed out opens sign-in and starts no process**; minimum window size 960 × 620 without clipping; appearance chosen in game shown by the launcher.

## Not verified here — needs a Windows PC, hardware or configuration

| Item | Why / what to check |
| --- | --- |
| Windows runtime of the launcher | Build the installer from CI (`stabled-windows-installer` artifact) and run it on Windows 10/11: WebView2 rendering (including the frosted backdrop blur, which WebKitGTK without compositing didn't render here), window sizing with display scaling at 125%/150% |
| Microsoft sign-in end to end | Needs a `MICROSOFT_CLIENT_ID` approved for the Minecraft services API. Check: browser opens, cancel/timeout, Java owner vs. account without Java, profile avatar, restart restores the session, sign-out removes `secure\account.bin` |
| DPAPI round trip | Sign in, restart, confirm the session restores; confirm `account.bin` is unreadable as text |
| System information | DXGI GPU names and VRAM, CPU name from the registry, installed RAM, Windows 11 naming |
| PC Optimiser | Each action's detect → Apply → Restore on Windows; UAC prompt for GPU scheduling, including declining it |
| Launch on Windows | `javaw.exe` without a console, after-launch Minimise/Hide/Keep, Force close, crash reporting |
| Real Discord | With `DISCORD_APPLICATION_ID` set and Discord running: activity named "Stabled Client" with the `stabled` art asset; Discord closed → game unaffected, status shows retrying |
| Real GPUs | Motion blur and menus on NVIDIA, AMD and Intel drivers, OpenGL and Vulkan |
| Live news | Mojang release notes (the host was blocked here; the failure/offline path was exercised) and a configured `NEWS_FEED_URL` |
| Custom background upload | The upload goes through the native file dialog, which wasn't driven here; the stored-image display, focal point and dimming paths were exercised |

## Running the checks yourself

```bash
cd launcher/src-tauri && cargo test --lib                     # backend unit tests
cd launcher && npm run typecheck                              # frontend types
cd launcher/src-tauri && cargo check --target x86_64-pc-windows-msvc

# Install a Save and print its launch command (development only, offline profile)
cd launcher/src-tauri && cargo build --features dev-tools
STABLED_DATA_DIR=/tmp/stabled target/debug/stabled --dev create "Dev Save"
STABLED_DATA_DIR=/tmp/stabled target/debug/stabled --dev command <id>
```
