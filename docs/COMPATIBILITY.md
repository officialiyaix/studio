# Compatibility

Verified on 2026-11-14 against Mojang's version manifest, `meta.fabricmc.net`, the official `FabricMC/fabric-example-mod` 26.3 branch and the Modrinth API.

## Supported combination

| Component | Version | Notes |
| --- | --- | --- |
| Minecraft: Java Edition | **26.3** (released 2026-09-15) | Unobfuscated; requires Java 25 |
| Java runtime | Mojang `java-runtime-epsilon` (Java 25) | Downloaded from Mojang's runtime manifest, shared by Saves |
| Fabric Loader | 0.20.1 (stable) | Launch profile from `meta.fabricmc.net` |
| Stabled Client | 1.0.0 | Bundled with the launcher; required |
| Build toolchain | Fabric Loom 1.19 (non-remapping), Gradle 9.8.2, JDK 25 | As in the official 26.3 example mod |

### Pinned performance stack (`launcher/src-tauri/stack/26.3.json`)

Downloaded from Modrinth's CDN onto the user's computer at install time and verified by SHA-512. Stabled does not redistribute these files.

| Mod | Version | Channel | Role | Licence | Required |
| --- | --- | --- | --- | --- | --- |
| Fabric API | 0.166.2+26.3 | release | Required by the Stabled client | Apache-2.0 | yes |
| Sodium | 0.9.3-beta.5+mc26.3 | **beta** | Rendering performance | Polyform Shield 1.0.0 | no |
| Lithium | 0.26.3+mc26.3 | release | Game-logic / tick performance | LGPL-3.0-only | no |
| FerriteCore | 9.0.0 | release | Memory use | MIT | no |
| ImmediatelyFast | 1.17.1+26.3 | release | HUD/text/immediate-mode rendering | LGPL-3.0-or-later | no |
| EntityCulling | 1.12.1 | release | Skips hidden (block) entities | tr7zw Protective License | no |

Sodium has no full release for 26.3 yet; the newest build is a beta (published 2026-11-12). It is labelled "Beta" in the mods list and in Create Save. For a fully stable stack, a 26.2 stack could be added (Sodium 0.9.2 is a release there), but 26.2 has not been implemented or tested with the Stabled client.

Mod Menu and Iris are **not** in the stack: Mod Menu needs an extra dependency, and this Sodium build declares itself incompatible with Iris ≤ 1.12.0 (the current Iris for 26.3 is 1.11.7). Users may add their own mods; Stabled's metadata checks will flag such declared conflicts.

## Renderers

| Renderer | Status |
| --- | --- |
| OpenGL backend (default on Windows) | Tested: menus, introduction, FPS display, motion blur (Mesa llvmpipe, GL 4.5 core) |
| Vulkan backend | Tested: same features (Mesa lavapipe, Vulkan 1.4) |
| Iris shader packs | Not in the stack. If a user installs Iris and a shader pack is active, motion blur is disabled with an explanation, because shader packs replace the renderer it runs in |
| Graphics API switch | In Stabled Settings → Performance; takes effect after restarting the game (labelled) |

Testing used software renderers in a Linux container; see [TESTING.md](TESTING.md) for what still needs a Windows PC with a real GPU.

## Unsupported

- Every Minecraft version other than 26.3 (listed as "Not supported yet" in Create Save; not selectable).
- Forge, NeoForge and Quilt mods (detected and flagged).
- The launcher and PC Optimiser target Windows 10/11 x64. Linux builds exist only for development (no persistent sign-in, optimiser unavailable). macOS is not supported.
