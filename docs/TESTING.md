# Testing

This records what has actually been exercised, how, and what still needs a Windows PC, real hardware or the external configuration values. Screenshots from these runs are in [`screenshots/`](screenshots/).

## Environment used

A Linux container with Xvfb and Mesa software rendering (llvmpipe for OpenGL 4.5, lavapipe for Vulkan 1.4), JDK 25 (plus Mojang's Java 17, 21 and 25 runtimes for the games), Rust 1.97 and Node 22. Network access to Mojang, Fabric and Modrinth; no access to Microsoft account services, Discord or Mojang's launcher-content (news) host.

## Verified

### Every supported Minecraft version (1.20.1 – 26.3)

All 23 Stabled client jars compile against their own Minecraft and Fabric API versions (`./gradlew build`). Each version was then run end to end with `tools/client-smoke-test.sh <version>`, which uses the launcher's own install pipeline and launch command:

1. create a Save for the version and install it (Mojang files, the version's Java runtime, Fabric, the pinned stack, the bundled Stabled jar);
2. generate a flat world with that version's dedicated server;
3. start the game: the introduction must open on the title screen and complete through Next ×4 and Finish (keyboard), then the Stabled title screen;
4. start again with quick play into the world, with motion blur on: FPS display, motion blur while turning, Stabled pause menu, Right Shift → Stabled Settings;
5. scan the logs for mixin, injection, linkage and Stabled errors.

The per-version results are in [COMPATIBILITY.md](COMPATIBILITY.md#test-results). Two launcher bugs found this way were fixed: identical files in one download batch (asset objects listed under several names) raced on one temporary file, and temporary names replaced the file extension, so `java.exe` and `java.dll` in a Java runtime shared `java.part` — which would have broken Java installs on Windows.

### In-game client in depth (real Minecraft 26.3 + Fabric 0.20.1 + the full pinned stack)

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
| Unit tests | Fabric version predicates; version-aware `breaks`; launch refused without a session (`SIGN_IN_REQUIRED`/`AUTH_UNCONFIGURED`); account debug output never contains tokens; download temporary names never collide |
| Install pipeline | Real downloads: Mojang version JSON, client jar, 73 libraries, 5,149 assets, Java 25 runtime, Fabric profile; SHA-1/SHA-512 verified; managed stack copied into the Save's own `mods` folder; second Save installs from the shared cache in seconds with its own copies |
| Mod checks | Correctly reports nothing for the pinned stack; correctly reads Sodium's declared incompatibility with Iris ≤ 1.12.0 |

### Launcher UI (the real Tauri app, WebKitGTK)

Startup sign-in popup over the blurred launcher (with the honest "not configured" state), dismissal, no re-prompt during navigation; Home, Saves (Overview/Mods/Save settings), PC Optimiser, Settings; navigation rail hover expansion over content without layout shift; theme switching (Stabled Night, Cloud, Sunset, Custom Background) and Simple/Frosted Glass applied instantly and persisted; Create Save field order, version list from Mojang's live manifest with supported/unsupported marking, inline validation (empty and duplicate names), creation; Install with live progress → Ready; **Launch while signed out opens sign-in and starts no process**; minimum window size 960 × 620 without clipping; appearance chosen in game shown by the launcher.

### Store, logs, crash help, backups, compatibility and emotes

| Area | Result |
| --- | --- |
| Unit tests | Crash explanations (out of memory, Fabric's incompatible-mod report with its fixes quoted, Mixin failure naming the mod, AMD driver crash from an `hs_err` file, mods in a stack trace, unknown crash at low confidence, a normal log reported as no crash); streamer-mode log redaction (name, user folder, server address, IP); world backup zip contents (`session.lock` excluded); optimiser actions well formed |
| Store | Live Modrinth search for a 1.20.1 Save (18,133 Fabric mods), detail view with gallery, mods Stabled installed shown as Installed via `version_files`, real install of Iris into the Save with its SHA-512 checked; installing BSL Shaders into a 1.20.2 Save without Iris put the pack in `shaderpacks` and added Iris for 1.20.2 |
| Logs | Latest log loaded with level colours and counts; Ctrl+F search with "2 of 8" and next/previous; Explain on a normal log reports no crash |
| Backups | Back up now wrote a zip; Restore added "TestWorld (restored …)" whose files are byte-identical to the original; the original world untouched |
| Find the broken mod | Test copy created without worlds; each round enabled exactly the expected mods in the copy; result screen named the mod; closing deleted the copy and left the real Save's mods unchanged |
| Compatibility | A Forge jar and an OptiFabric jar next to Sodium were reported (other loader, wrong Minecraft version, known conflict) |
| PC Optimiser | Apply all and the score ring on Linux (the Java launch tweak); its garbage-collector flags were then used for real launches of 1.20.1, 1.21.4 and 26.3, with no errors |
| Emotes | Wheel opened with B on 1.20.1, 1.21.4 and 26.3; hold-and-release, tap-and-click and number keys play the emote in third person (front) with the HUD pill; poses checked for each emote; smoke tests of 1.21.1, 1.21.9, 1.21.11 and 26.1.2 load the player model mixin without errors |
| Game modes (CPVP) | The five chosen 1.21.11 jars match the pinned SHA-512s byte for byte. A new 1.21.11 Save starts in CPVP and Fabric loads Tier Tagger, Shield Statuses, Shield Fixes, Totem Tweaks, In-Game Account Switcher, ukulib, WalksyLib, Mod Menu and Cloth Config with no errors (Tier Tagger's tier lookups were blocked by this sandbox's network). CPVP Saves on 1.20.1, 1.21.4, 1.21.8, 26.1.2 and 26.3 load their own builds without errors. Tier Tagger's "Obsolete Minecraft version" screen appeared on versions its author marks obsolete, so it's left out there and 1.21.4 was re-tested without it. In the launcher, switching a 1.20.1 Save Standard → CPVP added exactly the mode's seven files and CPVP → Standard removed exactly those, leaving the performance mods and the player's Iris; the header pill and Overview cards both switch. The title screen places Mod Menu's and In-Game Account Switcher's buttons in the Stabled panel |
| Launcher UI | Floating navigation rail with the Store; Home greeting; Logs and Backups tabs; Discord confirmation; Settings with Accounts first (red when signed out), streamer mode with the header indicator, fake specs in the specs bar |

## Not verified here — needs a Windows PC, hardware or configuration

| Item | Why / what to check |
| --- | --- |
| Windows runtime of the launcher | Build the installer from CI (`stabled-windows-installer` artifact) and run it on Windows 10/11: WebView2 rendering (including the frosted backdrop blur, which WebKitGTK without compositing didn't render here), window sizing with display scaling at 125%/150% |
| Microsoft sign-in end to end | Needs a `MICROSOFT_CLIENT_ID` approved for the Minecraft services API. Check: browser opens, cancel/timeout, Java owner vs. account without Java, profile avatar, restart restores the session, sign-out removes `secure\account.bin` |
| DPAPI round trip | Sign in, restart, confirm the session restores; confirm `account.bin` is unreadable as text |
| System information | DXGI GPU names and VRAM, CPU name from the registry, installed RAM, Windows 11 naming |
| PC Optimiser | Each action's detect → Apply → Restore on Windows (mouse acceleration and Sticky Keys apply immediately); Apply all asking for administrator permission once for GPU scheduling and network responsiveness, including declining it; High priority for Minecraft visible in Task Manager |
| Emotes seen by others | Emotes have no server component, so other players don't see them; nothing to test beyond the local view |
| Launch on Windows | `javaw.exe` without a console, after-launch Minimise/Hide/Keep, Force close, crash reporting |
| Real Discord | With `DISCORD_APPLICATION_ID` set and Discord running: activity named "Stabled Client" with the `stabled` art asset; Discord closed → game unaffected, status shows retrying |
| Real GPUs | Motion blur and menus on NVIDIA, AMD and Intel drivers, OpenGL and Vulkan — at least one version per motion blur implementation (e.g. 1.20.1, 1.21.8 and 26.3) |
| Live news | Mojang release notes (the host was blocked here; the failure/offline path was exercised) and a configured `NEWS_FEED_URL` |
| Custom background upload | The upload goes through the native file dialog, which wasn't driven here; the stored-image display, focal point and dimming paths were exercised |

## Running the checks yourself

```bash
cd launcher/src-tauri && cargo test --lib                     # backend unit tests
cd launcher && npm run typecheck                              # frontend types
cd launcher/src-tauri && cargo check --target x86_64-pc-windows-msvc

# Install a Save and print its launch command (development only, offline profile)
cd launcher/src-tauri && cargo build --features dev-tools
STABLED_DATA_DIR=/tmp/stabled target/debug/stabled --dev create "Dev Save" 1.21.11
STABLED_DATA_DIR=/tmp/stabled target/debug/stabled --dev command <id>

# Build the client for some versions and run the in-game smoke test (Linux, Xvfb)
cd client && ./gradlew -Pstabled.versions=1.20.1,26.3 build
tools/client-smoke-test.sh 1.20.1      # screenshots and logs in ./smoke-test/1.20.1/
```
