# Reference research

Snapshot: shallow clones of each project's default branch, November 2026.
Nothing below was copied into Novastrap. These notes record ideas and facts. The code is our own.

## Summary table

| Project | Platform | Stack | License | What it actually is |
|---|---|---|---|---|
| bloxstraplabs/bloxstrap | Windows | C# / .NET 6 WPF (+ wpfui) | MIT | Replacement bootstrapper. Downloads Roblox packages itself, applies mods/FastFlags, Discord RPC. **No longer actively developed.** |
| fishstrap/fishstrap | Windows | Fork of Bloxstrap (C#/WPF) | MIT (+ Bloxstrap MIT notice) | Bloxstrap fork: Studio support, server location via RoValra API, cache cleaner, channel switcher, optional cookie access. |
| AppleBlox/appleblox | macOS | Svelte 4 + Tailwind + bits-ui on NeutralinoJS | **GPL-3.0** | macOS launcher with a polished sidebar UI, Discord RPC, FastFlags, mods. |
| SpeeNotPee/Pigment | Linux | Rust workspace, GTK4/libadwaita | MIT | GUI front-end for Sober: typed config, profiles, FastFlags, mods via Sober's overlay. |
| JADRT22/soberix | Linux | Python (GTK) | MIT | Another Sober manager: quality profiles, allow-listed FastFlag presets, backups, activity history, CLI. |
| Novetus/Novetus_src | Windows | C# WinForms + Lua | MIT (bundled third-party parts vary) | Runs **old, archived Roblox clients** offline/LAN. Unrelated to the modern client. |
| vinegarhq/sober | Linux | Closed source | n/a (issue tracker only) | Runs the Android Roblox client on Linux. The repo has **no runtime source**, only a README and an icon. |

## Per-project notes

### Bloxstrap (the main technical reference for Windows)
- **Version lookup:** `GET https://clientsettingscdn.roblox.com/v2/client-version/WindowsPlayer[/channel/{name}]`. If the CDN fails, it falls back to `clientsettings.roblox.com`. The response holds `version` and `clientVersionUpload` (the version GUID).
- **Install:** it downloads `{guid}-rbxPkgManifest.txt` from `setup.rbxcdn.com` (it tests mirrors `setup-aws`/`setup-ak` for connectivity), then each `.zip` package. It checks each package's **MD5 against the manifest** and extracts with a hard-coded package→folder map (`content-fonts.zip` → `content\fonts\`, …). It also writes `AppSettings.xml`.
- **Cache:** packages are cached by hash, and old hashes are pruned after an upgrade.
- **Registry:** it registers itself as the `roblox-player:` / `roblox:` protocol handler, and it writes the channel under `HKCU\SOFTWARE\ROBLOX Corporation\Environments\...`.
- **Activity tracking:** it tails `%LOCALAPPDATA%\Roblox\logs\*.log` and matches `[FLog::Output] ! Joining game '<jobId>' place <placeId> at <ip>`, `Replicator created`, disconnect lines and so on. Discord RPC and server location (ipinfo.io) both use this.
- **Settings:** JSON files in its own app-data folder (`JsonManager<T>`). FastFlags go to `ClientSettings\ClientAppSettings.json` inside the version folder.
- **Limitations:** WPF is Windows-only and dated. The bootstrapper is a 1,600-line class. The package map is hard-coded, so Roblox package changes break it. The project is unmaintained.

### Fishstrap
- Moves the package map to a **remote JSON** (`config.fishstrap.app`), so map changes don't need a release. That trades robustness for depending on a server.
- **CookiesManager** reads `.ROBLOSECURITY` from Roblox's own `LocalStorage\RobloxCookies.dat`. It's opt-in and host-restricted to the Roblox domain. **Novastrap will not do this.** It is the session token, and the brief rules out cookie handling.
- Server region data comes from the third-party RoValra API.

### AppleBlox
- The best UI reference among the launchers: page-per-section (`Home`, `Engine`, `Appearance`, `Integrations`, `Mods`, `Account`…), a separate bootstrapper window, and a shared `components/ui` kit.
- **GPL-3.0:** we must not copy any of its code into Novastrap unless we also adopt the GPL. We only take the layout idea (sidebar plus section pages plus a separate launch window).

### Pigment
- The cleanest architecture: `pigment-core` (config, profiles, presets, mods, protocol, roblox_api, activity, discord), `pigment-launch` (a tiny protocol-handler binary) and `pigment` (UI). **We adopt this core/launch/UI split.**
- Its protocol takeover is opt-in and reversible. We copy that policy.
- FastFlags use Bloxstrap's JSON format, so presets are portable.

### Soberix
- FastFlag presets restricted to the **post-2025-09-30 allowlist**. Roblox now ignores non-allowlisted flags in the player.
- Backups and activity history are good ideas for the Home "recent activity" panel.

### Novetus
- Not applicable. It runs archived clients against its own Lua server scripts. Nothing in it helps launch the modern client, and bundling old Roblox binaries raises copyright problems. **Out of scope.**

### Sober
- Closed source. It's only relevant if we ever add Linux support: we would drive Sober the way Pigment and Soberix do.

## Licensing conclusions
- Novastrap will be **MIT**. We may *read* the MIT projects and re-implement ideas. If we ever port a non-trivial piece of MIT code, we keep its copyright notice in `THIRD_PARTY_NOTICES.md`.
- **No AppleBlox (GPL-3.0) code** goes in. Only general UX ideas.
- No logos, icons or images from any reference. Roblox trademarks are used only to describe what the app works with.

## Features worth taking (re-implemented)
1. Version lookup through clientsettings, with channel support (Bloxstrap).
2. Hash-verified, resumable package installs (Bloxstrap), but with a **data-driven package map** shipped in-app and validated, not fetched from a third party.
3. Log-tail activity watcher → Discord RPC, recent activity, rejoin (Bloxstrap).
4. Allowlist-only FastFlag editor with presets (Soberix and Fishstrap).
5. Profiles that apply on launch (Pigment).
6. A core/launch/UI separation (Pigment).
7. An opt-in, reversible protocol handler (Pigment).

## Weaknesses we can improve on
- Monolithic bootstrapper classes → small services with tests.
- Hard-coded package maps → a versioned map plus clear failure when an unknown package appears.
- Dated WPF UI → a modern web-tech UI with animations and theming.
- No discovery or server browsing in the Windows launchers → Steam-style Discover and Servers pages.
- Cookie scraping (Fishstrap) → official OAuth 2.0.
