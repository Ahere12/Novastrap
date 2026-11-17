# Novastrap architecture

## Audit result
`github.com/Ahere12/Novastrap` had **no commits** when this work started (Nov 2026). There was no existing stack to keep, so the stack below is a fresh choice.

## Stack decision: Tauri 2 (Rust) + React + TypeScript

| Option | Verdict |
|---|---|
| **Tauri 2 + React/TS** | **Chosen.** Small installer (~10 MB) and fast cold start, using WebView2, which ships with Windows 11. The Rust backend suits downloads, hashing, the registry, process control and OS keychain storage. The web UI gives us the animations and theming the design calls for. |
| C# WPF (Bloxstrap's stack) | Proven for this job, but the UI is hard to make feel modern, and the stack is Windows-only. |
| Electron | Easiest UI, but a 150 MB+ install and slower start. It conflicts with "fast startup". |
| Neutralino (AppleBlox) | Light, but its native API is thin. We would still need sidecar binaries for the hard parts. |

Frontend libraries are kept to a minimum: React, React Router, Zustand (state), Zod (API validation), and Framer Motion (transitions, added in the UI phase). There is no component framework. The design system is our own CSS.

## Process layout

```
┌────────────────────────── novastrap.exe (Tauri) ──────────────────────────┐
│  WebView (React UI)                                                       │
│   pages/ Home · Discover · Servers · Accounts · Client · Mods · Settings  │
│   ui/    design-system components                                         │
│   api/   typed `invoke()` wrappers  ← the only way the UI reaches backend │
├───────────────────────────────── IPC (commands/events) ───────────────────┤
│  Rust backend: src-tauri (Tauri glue) + core/ (novastrap-core services)   │
│   commands/   thin Tauri command handlers (validation → service call)     │
│   roblox/     detection · deployment (version API) · installer · launcher │
│   web/        HTTP client: cache · rate limit · retry · timeout · schemas │
│   auth/       OAuth 2.0 PKCE · token store (Windows Credential Manager)   │
│   config/     settings + profiles (JSON, versioned, atomic writes)        │
│   fflags/     allowlist + ClientAppSettings.json writer                   │
│   activity/   log-tail watcher → events (RPC, history, rejoin)            │
│   integrations/ discord rpc · notifications                               │
│   updater/    signed self-update (tauri-plugin-updater, minisign keys)    │
│   logging/    rotating file logs with secret redaction                    │
└───────────────────────────────────────────────────────────────────────────┘
```

Rules:
- The UI never makes Roblox HTTP calls itself. Everything goes through Rust, so caching, rate limits and validation live in one place, and the CSP can block other origins.
- Command handlers stay thin. Logic lives in services with unit tests.
- Services return `Result<T, AppError>`. `AppError` serializes to `{ kind, message, retryable }` so the UI can show a meaningful error.

## Roblox integration design

### Detection
1. `HKCU\Software\Classes\roblox-player\shell\open\command` shows who owns the protocol (the official client, Bloxstrap, or us).
2. The official install is at `%LOCALAPPDATA%\Roblox\Versions\version-*\RobloxPlayerBeta.exe`. The installed version is read from that folder name, or from `AppSettings.xml`.
3. The latest version comes from `clientsettingscdn.roblox.com/v2/client-version/WindowsPlayer`, with `clientsettings.roblox.com` as the fallback.
4. Status: `NotInstalled | UpToDate | UpdateAvailable | Unknown(error)`.

### Install and update (two strategies, picked in that order)
- **A. Official installer (default, ships first).** Download `RobloxPlayerInstaller.exe` from `setup.rbxcdn.com`. Before running it, **verify its Authenticode signature** and require "Roblox Corporation" as the signer. Then run it and wait. This uses Roblox's own distribution untouched.
- **B. Managed install (later, opt-in).** Bloxstrap-style: fetch the manifest, download packages with resume support and MD5 checks, and stage into `versions/{guid}.partial`. A single atomic rename completes the install, so an interrupted run never leaves a half-written version. This strategy enables channels and profiles per install.

In both strategies, the executable is never modified.

### Launch
- **Join a game or server:** `roblox://experiences/start?placeId=…[&gameInstanceId=…]` is Roblox's documented deep-link format. We hand it to the installed client through `ShellExecute`. The client uses **its own** login session, so Novastrap never touches session tokens.
- **Protocol takeover (opt-in, reversible):** if the user enables it, Novastrap registers as the `roblox-player:`/`roblox:` handler. It then applies the active profile (FastFlags and settings) and forwards the link to the real client. The previous handler value is saved so it can be restored.

### Accounts
- **Roblox OAuth 2.0 with PKCE** (an official, supported flow). We get the scopes `openid profile`, which give the user ID, username, display name and avatar.
- Tokens live in **Windows Credential Manager** through the `keyring` crate. They are never written to JSON or logs.
- **Limitation:** OAuth signs in the *launcher*. It does **not** change which account the Roblox *client* uses. True client account switching would need the session cookie, which is exactly what we refuse to handle. So "switching" changes the profile Novastrap displays, and the in-game account is managed in Roblox itself.
- Each developer has to register an OAuth app at create.roblox.com to get a client ID. That's a one-time manual step.

### Web API layer (`web/`)
- One `reqwest` client with a 10 s timeout and a user-agent of `Novastrap/<version>`.
- A per-host token-bucket rate limiter. On 429 it backs off exponentially with jitter and honors `Retry-After`. It retries idempotent GETs only, at most 3 times.
- A TTL cache in memory plus on disk (thumbnails 24 h, game details 5 min, server lists 30 s, version 5 min). Stale data can be served when the network is down, and it's labelled as stale.
- Every response is deserialized into a typed `serde` struct. A schema mismatch becomes `AppError::UpstreamChanged`. Bad data never reaches the UI.

| Feature | Endpoint | Auth | Documented? |
|---|---|---|---|
| Client version | clientsettingscdn `/v2/client-version` | none | public, unofficial-stable |
| Game details | games.roblox.com `/v1/games?universeIds=` | none | Web API docs |
| Place → universe | apis.roblox.com `/universes/v1/places/{id}/universe` | none | Web API docs |
| Public servers | games.roblox.com `/v1/games/{placeId}/servers/Public` | none | Web API docs |
| Thumbnails | thumbnails.roblox.com `/v1/games/icons`, `/v1/batch` | none | Web API docs |
| Users / avatar | users.roblox.com `/v1/users/{id}`, thumbnails headshot | none | Web API docs |
| Discover sorts / search | apis.roblox.com explore-api / search-api | none | **Undocumented**, see risks |
| Profile | OAuth `userinfo` | OAuth | **Official** (Open Cloud) |

### Storage
- `%APPDATA%\Novastrap\settings.json`: versioned schema with migrations, written atomically (write to a temp file, then rename).
- `%APPDATA%\Novastrap\profiles\*.json`: FastFlags and launch settings per profile. These can be imported and exported.
- `%LOCALAPPDATA%\Novastrap\cache`, `\logs`, `\downloads`.
- Secrets go only in Credential Manager.

### FastFlags
- Only flags on the **post-2025-09-30 allowlist** are offered. The allowlist ships as data (`fflags/allowlist.json`), each entry with a type, range and description.
- Unknown flags in an imported file are listed and dropped, never silently written.
- The UI makes no claims about performance. Each flag shows what it does, not "+FPS".

## Crate layout (decided in phase 1)
- `core/` (`novastrap-core`) holds all services: Roblox, web, config, auth. It has **no Tauri dependency**, so `cargo test --manifest-path core/Cargo.toml` runs quickly on any toolchain. A future tiny protocol-handler binary can reuse it (Pigment's `pigment-launch` idea).
- `src-tauri/` is the desktop app. It contains only Tauri commands, the plugin setup and state.
- Reason for the split: linking Tauri into a unit-test binary on Windows fails without an app manifest (`STATUS_ENTRYPOINT_NOT_FOUND`). Keeping the logic Tauri-free avoids that, and it enforces the service boundary anyway.
- The library is `rlib` only, since the app is desktop-only. The template's `cdylib` exceeds GNU ld's export limit and only matters for mobile.
