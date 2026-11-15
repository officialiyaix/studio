# Stabled

Stabled is a performance-focused Windows launcher for **Minecraft: Java Edition** with its own in-game client.

- **Saves** — isolated Minecraft installations, each with its own mods, settings, worlds, resource packs, shader packs, screenshots and logs.
- **Stabled Client** — a Fabric mod that adds the Stabled main and pause menus, Stabled Settings, a five-step first-launch introduction, a real FPS display, real motion blur and Discord Rich Presence.
- **PC Optimiser** — a small set of reversible, game-relevant Windows settings that change only when you choose Apply.
- **Microsoft sign-in** through the system browser (no password form), with the session encrypted by Windows DPAPI.
- **Nine themes** (eight built-in plus Custom Background), **pre-themes**: a gallery of 24 backgrounds in 8 categories, in the launcher and in game, and **Simple / Frosted Glass** surfaces, shared by the launcher and the in-game menus.

Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft.

## Repository layout

| Path | What it is |
| --- | --- |
| `launcher/` | Tauri 2 desktop app: React + TypeScript UI (`src/`) and Rust backend (`src-tauri/`) |
| `client/` | The Stabled Client Fabric mod, built for every supported Minecraft version from one source tree (Gradle, Fabric Loom) |
| `shared/themes.json` | The nine theme choices, used by both the launcher and the mod |
| `config/stabled.config.json` | External configuration values (see below) |
| `branding/` | Original Stabled mark and wordmark (SVG sources and PNG renders) |
| `docs/` | Architecture, compatibility, configuration, file contracts, news format, testing |

## Supported versions

Every Minecraft: Java Edition release from **1.20.1 to 26.3** — 23 versions — on Fabric Loader 0.20.1:

| Minecraft | Java | Stabled client |
| --- | --- | --- |
| 1.20.1, 1.20.2, 1.20.3, 1.20.4 | 17 | 1.0.0 |
| 1.20.5, 1.20.6, 1.21, 1.21.1 – 1.21.11 | 21 | 1.0.0 |
| 26.1, 26.1.1, 26.1.2, 26.2, 26.3 | 25 | 1.0.0 |

Each version has its own pinned performance stack (Fabric API, Sodium, Lithium, FerriteCore, ImmediatelyFast, EntityCulling where a compatible build exists) and its own Stabled client jar. Exact versions, beta components, gaps and what was tested on each version are in [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md). Create Save lists older releases as not supported; snapshots and pre-releases are not offered.

## Configuration

Four values are configured in [`config/stabled.config.json`](config/stabled.config.json) or as same-named environment variables at build time:

| Value | Needed for | When missing |
| --- | --- | --- |
| `MICROSOFT_CLIENT_ID` | Microsoft sign-in (and therefore Launch) | Sign-in shows "not configured"; everything else works |
| `DISCORD_APPLICATION_ID` | Discord Rich Presence ("Stabled Client") | Presence reports "not configured" and sends nothing |
| `DISCORD_INVITE_URL` | Settings → Socials → Join Discord | The button is disabled with an explanation |
| `NEWS_FEED_URL` | Stabled news in the news panel | Minecraft release notes and built-in guides are shown |

`MICROSOFT_CLIENT_ID` (Stabled's own Microsoft app; Minecraft sign-in also needs Mojang's approval of it), `DISCORD_APPLICATION_ID` and `DISCORD_INVITE_URL` are set in `config/stabled.config.json`; `NEWS_FEED_URL` is not. See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for how to obtain each one.

## Building

Requirements: Node.js 22, Rust (stable), JDK 25 (Gradle runs on 25 and compiles each Minecraft version for Java 17, 21 or 25). On Windows, the WebView2 runtime (installed automatically by the installer).

```bash
# 1. Build the Stabled Client mod for every Minecraft version and bundle the jars into the launcher
cd launcher
npm ci
npm run bundle-client                  # all 23 versions → src-tauri/resources/clients/stabled-client-<minecraft>.jar
npm run bundle-client -- 26.3 1.21.11  # or only some versions

# 2. Run in development
npm run tauri dev

# 3. Build the Windows installer (on Windows)
npx tauri build              # → src-tauri/target/release/bundle/nsis/Stabled_1.0.0_x64-setup.exe
```

CI (`.github/workflows/build.yml`) builds the mod for every Minecraft version in parallel, runs the backend tests, type-checks the Windows-only code and produces the NSIS installer on `windows-latest`.

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Compatibility and performance stack](docs/COMPATIBILITY.md)
- [Configuration](docs/CONFIGURATION.md)
- [Launcher ↔ game file contracts](docs/CONTRACTS.md)
- [News feed format](docs/NEWS_FORMAT.md)
- [Testing: what was verified, and what still needs a Windows PC](docs/TESTING.md)
