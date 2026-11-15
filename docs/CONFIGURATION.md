# Configuration

Stabled reads four external values. They are public identifiers, not secrets, but they belong to whoever publishes Stabled, so none are filled in here.

Set them in one of three places (later wins):

1. `config/stabled.config.json` in the repository (baked in at build time),
2. environment variables with the same names when running `npx tauri build` (CI uses repository *variables* for this),
3. `%APPDATA%\Stabled\stabled.config.json` on a user's PC (runtime override, same keys).

Malformed values are treated as missing. Missing values disable only the feature that needs them, and the UI says "not configured" — it never pretends the connection works.

## `MICROSOFT_CLIENT_ID`

Needed for Microsoft sign-in and therefore for launching Minecraft.

1. Register an application in the [Microsoft Entra admin center](https://entra.microsoft.com/) → App registrations → New registration.
   - Supported account types: **Personal Microsoft accounts only** (Stabled uses the `consumers` tenant).
   - Platform: **Mobile and desktop applications**, redirect URI **`http://localhost`** (any port is accepted for loopback).
   - Allow public client flows: **Yes**. No client secret is used (PKCE).
2. New applications must be approved by Mojang before they can call the Minecraft services API (`api.minecraftservices.com`). Request access through Mojang's current process for third-party launchers before release — check Mojang's/Microsoft's developer guidance for the up-to-date form, as it has moved over time. Until approval, sign-in reaches Microsoft and Xbox Live but `login_with_xbox` is rejected; Stabled reports this as an error explaining that the application may not be approved.
3. Put the Application (client) ID — a GUID — in `MICROSOFT_CLIENT_ID`.

Scopes requested: `XboxLive.signin offline_access`.

## `DISCORD_APPLICATION_ID`

Needed for Discord Rich Presence.

1. Create an application in the [Discord Developer Portal](https://discord.com/developers/applications) named **Stabled Client** — Discord shows the application's name as the activity title ("Playing Stabled Client").
2. Under Rich Presence → Art Assets, upload the Stabled mark (`branding/mark-1024.png`) with the key **`stabled`**; the client uses it as the large image.
3. Put the Application ID (a 17–20 digit number) in `DISCORD_APPLICATION_ID`.

The launcher passes it to the game as `-Dstabled.discordAppId`. No Discord login, token or password is ever involved — presence talks only to the Discord desktop app on the same PC.

## `DISCORD_INVITE_URL`

The Stabled community invite opened by Settings → Socials → Join Discord, e.g. `https://discord.gg/<code>`. Only `https` links on `discord.gg` or `discord.com` are accepted.

## `NEWS_FEED_URL`

Optional `https` URL of a Stabled news feed in the format described in [NEWS_FORMAT.md](NEWS_FORMAT.md). Without it, the news panel shows Minecraft: Java Edition release notes from Mojang (if enabled in Settings) and Stabled's built-in guides.

## Other identifiers to review before publishing

- `launcher/src-tauri/tauri.conf.json` → `identifier` (`app.stabled.launcher`) and `bundle.publisher`. Change the identifier to a reverse-DNS name you control before the first public release; it determines Windows install paths.
- Code-signing for the installer is not configured. Unsigned installers trigger SmartScreen warnings.
