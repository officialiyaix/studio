# Stabled

Stabled is a performance-focused Windows launcher for **Minecraft: Java Edition** with its own in-game client.

- **Saves** — isolated Minecraft installations, each with its own mods, settings, worlds, resource packs, shader packs, screenshots and logs.
- **Stabled Client** — a Fabric mod that adds the Stabled main and pause menus, Stabled Settings, a five-step first-launch introduction, a real FPS display, real motion blur and Discord Rich Presence.
- **PC Optimiser** — a small set of reversible, game-relevant Windows settings that change only when you choose Apply.
- **Microsoft sign-in** through the system browser (no password form), with the session encrypted by Windows DPAPI.
- **Nine themes** (eight built-in plus Custom Background) and **Simple / Frosted Glass** surfaces, shared by the launcher and the in-game menus.

Not an official Minecraft product. Not approved by or associated with Mojang or Microsoft.

## Repository layout

| Path | What it is |
| --- | --- |
| `launcher/` | Tauri 2 desktop app: React + TypeScript UI (`src/`) and Rust backend (`src-tauri/`) |
| `client/` | The Stabled Client Fabric mod for Minecraft 26.3 (Java 25, Gradle, Fabric Loom) |
| `shared/themes.json` | The nine theme choices, used by both the launcher and the mod |
| `config/stabled.config.json` | External configuration values (see below) |
| `branding/` | Original Stabled mark and wordmark (SVG sources and PNG renders) |
| `docs/` | Architecture, compatibility, configuration, file contracts, news format, testing |

## Supported versions

| Minecraft | Loader | Stabled client | Status |
| --- | --- | --- | --- |
| **26.3** | Fabric Loader 0.20.1 | 1.0.0 | Implemented and tested (OpenGL and Vulkan) |
| Any other version | — | — | Not supported; shown as "Not supported yet" in Create Save |

The pinned performance stack (Fabric API, Sodium, Lithium, FerriteCore, ImmediatelyFast, EntityCulling) and its exact versions are listed in [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md). Sodium for 26.3 is currently only published as a beta, and Stabled labels it as such.

## Configuration

Four values are configured in [`config/stabled.config.json`](config/stabled.config.json) or as same-named environment variables at build time:

| Value | Needed for | When missing |
| --- | --- | --- |
| `MICROSOFT_CLIENT_ID` | Microsoft sign-in (and therefore Launch) | Sign-in shows "not configured"; everything else works |
| `DISCORD_APPLICATION_ID` | Discord Rich Presence ("Stabled Client") | Presence reports "not configured" and sends nothing |
| `DISCORD_INVITE_URL` | Settings → Socials → Join Discord | The button is disabled with an explanation |
| `NEWS_FEED_URL` | Stabled news in the news panel | Minecraft release notes and built-in guides are shown |

None of these are set in this repository. See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for how to obtain each one.

## Building

Requirements: Node.js 22, Rust (stable), JDK 25. On Windows, the WebView2 runtime (installed automatically by the installer).

```bash
# 1. Build the Stabled Client mod and bundle it into the launcher
cd launcher
npm ci
npm run bundle-client        # runs ./gradlew build in /client and copies the jar

# 2. Run in development
npm run tauri dev

# 3. Build the Windows installer (on Windows)
npx tauri build              # → src-tauri/target/release/bundle/nsis/Stabled_1.0.0_x64-setup.exe
```

CI (`.github/workflows/build.yml`) builds the mod, runs the backend tests, type-checks the Windows-only code and produces the NSIS installer on `windows-latest`.

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Compatibility and performance stack](docs/COMPATIBILITY.md)
- [Configuration](docs/CONFIGURATION.md)
- [Launcher ↔ game file contracts](docs/CONTRACTS.md)
- [News feed format](docs/NEWS_FORMAT.md)
- [Testing: what was verified, and what still needs a Windows PC](docs/TESTING.md)
