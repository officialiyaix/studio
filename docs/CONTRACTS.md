# Launcher ↔ game contracts

The launcher and the Stabled Client share a few JSON files and JVM properties. Both sides preserve keys they don't know, and every write is atomic (temporary file + rename).

## JVM properties (set by the launcher)

| Property | Value |
| --- | --- |
| `stabled.home` | `%APPDATA%\Stabled` |
| `stabled.saveDir` | `%APPDATA%\Stabled\saves\<id>` |
| `stabled.discordAppId` | `DISCORD_APPLICATION_ID`, only when configured |

Without them (e.g. `gradlew runClient`) the client uses folders beside the game directory and says it is running in development mode.

## `appearance.json` (global, in `stabled.home`)

Written by the launcher (Settings → Appearance) and by the game (introduction step 1, Stabled Settings → Appearance). The launcher re-reads it when its window regains focus and when a game exits; the game re-checks it at most once a second while Stabled UI is drawn.

```json
{
  "version": 1,
  "theme": "stabled-night",        // one of the eight ids in shared/themes.json, or "custom"
  "surface": "simple",             // "simple" | "glass"
  "glassTransparency": 35,         // 10–90; higher = more background visible; kept while "simple"
  "customBackground": {            // null when no image has been uploaded
    "file": "backgrounds/custom.png",
    "focusX": 0.5, "focusY": 0.5,  // focal point for cover-fitting, 0–1
    "dim": 0.45,                   // readability overlay, 0–0.85
    "revision": 3                  // bumped on each upload so the game reloads the texture
  }
}
```

## `stabled/client.json` (per Save)

Created by the launcher with the Save defaults; edited by the launcher only while the Save is not running, and by the game while it runs.

```json
{
  "version": 1,
  "fps":        { "enabled": true, "position": "top-left", "scale": 1.0, "color": "theme", "background": "panel" },
  "motionBlur": { "enabled": false, "strength": 40 },
  "discord":    { "enabled": true, "showElapsed": true },
  "performance":{ "preset": "balanced", "applied": "balanced" },
  "menus":      { "styled": true }
}
```

- `fps.position`: `top-left | top-right | bottom-left | bottom-right`; `scale`: `0.75 | 1.0 | 1.25 | 1.5 | 2.0`; `color`: `theme | white | green | yellow | cyan`; `background`: `panel | shadow | none`.
- `motionBlur.strength`: 10–100.
- `performance.preset`: `performance | balanced | quality | custom`. `applied` is written by the game after it applies a preset, so a preset chosen in the launcher is applied once at the next launch and never re-applied over manual tweaks.

Documented defaults for a new Save: Stabled Night, Simple surfaces, Balanced preset, FPS display on, motion blur off, Discord Presence on.

## `stabled/onboarding.json` (per Save)

```json
{ "version": 1, "completed": false, "skipped": false, "step": 1, "completedAt": "2026-11-15T00:15:17Z" }
```

`step` is saved on every Next/Back so closing the game resumes at the same step. `completed` becomes true only on Finish or Skip. The launcher's "Show again next launch" resets it; in game, Stabled Settings → Menus & Introduction → Replay introduction does the same and opens it immediately.

On Skip, choices already made are kept (every control saves immediately); settings on steps not yet visited keep the Save's stored values — the documented defaults above, or whatever was set for this Save in the launcher before its first launch.

## `stabled/runtime.json` (per Save, written by the game)

Live status for the launcher. Ignored unless `pid` is the process the launcher started.

```json
{
  "version": 1, "pid": 20180, "updatedAt": "…",
  "discord": { "status": "connected", "detail": null },     // connected | connecting | waiting_for_discord | rejected | disabled | not_configured | stopped
  "motionBlur": { "available": true, "reason": null },
  "introductionOpen": false
}
```

## `stabled/managed.json` (per Save, launcher only)

Which files in `minecraft/mods` Stabled installed, their SHA-512, whether they are required, and whether the user disabled or removed them. See ARCHITECTURE.md → Mods.
