# Roadmap and development checklist

Every phase ends with the app building, the tests passing, and the features **really working**. No dead buttons ship: a section that isn't built yet shows as "coming in phase N" and has no controls.

## What is realistic

| Feature | Realistic? | Notes |
|---|---|---|
| Detect, install and update Roblox | ✅ | Official signed installer first, managed installs later |
| Launch games, deep links, protocol handler | ✅ | Documented `roblox://experiences/start` |
| Join a specific public server | ✅ | `gameInstanceId` from the public servers API |
| Server region **before** joining | ⚠️ Partial | Roblox's API gives no region. Only third-party APIs (RoValra) estimate it. We show region **after** joining, from the log IP, as an opt-in |
| Account profile (name, avatar, ID) | ✅ | Through OAuth 2.0 |
| Switching the account **inside Roblox** | ❌ | Needs the session cookie, which we refuse to handle. We switch the launcher identity only |
| Favorites / recently played from the Roblox account | ⚠️ | Needs authenticated web APIs, not OAuth scopes. Local history from the activity watcher instead |
| Discover: trending, popular, search | ⚠️ | Undocumented endpoints. Built behind an adapter with fallback |
| FastFlags | ✅ | Allowlisted flags only |
| Launcher themes, fonts, backgrounds, sounds | ✅ | Launcher-side only |
| In-game mods (cursor, sounds, fonts) | ⚠️ | Only by replacing content files in a managed install (Bloxstrap-style). Never touches the exe. Phase 7+ |
| Discord Rich Presence | ✅ | Log watcher plus the Discord IPC pipe |
| Signed self-update | ✅ | tauri-plugin-updater with minisign keys and GitHub Releases |
| macOS / Linux | Later | The Tauri stack allows it. Linux would drive Sober |

## Phases

### Phase 1: Foundation ← *current*
- [x] Audit the repo (empty) and research the references → `docs/RESEARCH.md`
- [x] Architecture → `docs/ARCHITECTURE.md`
- [x] Tauri 2 + React + TS scaffold, MIT license, `.gitignore`
- [x] `AppError` type, structured logging to file
- [x] Roblox detection service: protocol handler owner, installed versions, latest version (clientsettings + fallback)
- [x] App shell: sidebar navigation, Home shows real install status
- [x] Unit tests for version parsing and detection logic

### Phase 2: Web API layer
- [ ] Shared HTTP client: timeout, per-host rate limit, retry with backoff, `Retry-After`
- [ ] Memory and disk TTL cache with stale-while-offline
- [ ] Typed models: games, thumbnails, users, servers
- [ ] Tests against recorded fixtures (test-only, never shipped as data)

### Phase 3: UI system
- [ ] Design tokens, theming (dark default, accent colors), typography
- [ ] Component kit: Button, Card, GameTile, Skeleton, Toast, Modal, Tabs, Toggle, Slider
- [ ] Page transitions and loading screens; error and empty states
- [ ] Logo and app icon

### Phase 4: Install and launch
- [ ] Download the official installer, verify its Authenticode signature, run it, show progress
- [ ] Launch through the deep link; Quick Play (last game)
- [ ] Opt-in protocol takeover, with saved restore
- [ ] Interrupted-operation recovery; error messages for each failure kind

### Phase 5: Discover and Servers
- [ ] Discover sorts and search (adapter), game detail page
- [ ] Server browser: pagination, refresh, player counts, join a specific server

### Phase 6: Accounts and settings
- [ ] OAuth 2.0 PKCE sign-in; Credential Manager token store; multiple saved identities
- [ ] Settings pages (General, Appearance, Accounts, Game, Integrations, Downloads, Storage, Notifications, Privacy, About)
- [ ] Profiles; allowlisted FastFlag editor; import/export; reset

### Phase 7: Integrations
- [ ] Activity watcher (log tail); recent activity; rejoin
- [ ] Discord Rich Presence; Windows notifications
- [ ] Customization manager (themes, fonts, backgrounds, sounds)

### Phase 8: Hardening and release
- [ ] Managed install strategy (manifest, MD5, staging, atomic swap)
- [ ] Signed auto-update; code-signing certificate; installer (NSIS/MSI)
- [ ] Performance pass, accessibility pass, crash reporting (opt-in)

## Technical risks

| Risk | Impact | Mitigation |
|---|---|---|
| Undocumented Discover/search endpoints change or get blocked | Discover breaks | Adapter layer, schema validation, cached fallback, curated list fallback |
| Roblox changes installer, package map or deep-link format | Install or launch breaks | Strategy A uses Roblox's own installer; package map as versioned data; integration test run on a schedule |
| FastFlag allowlist changes | Flags silently ignored | Allowlist as data and updated with releases; "applied" means written, not "works" |
| Roblox rate limits (429) | Empty pages | Token bucket, caching, `Retry-After` |
| OAuth app requires registration or review | Sign-in unavailable in dev builds | App works fully signed-out; OAuth is an add-on |
| SmartScreen / antivirus flags an unsigned exe | Users scared off | Code-signing certificate before public release |
| WebView2 missing on older Windows 10 | App won't open | Tauri's bundled WebView2 bootstrapper in the installer |
| Roblox Terms of Use | Account risk for users | No injection, no Hyperion interaction, no cookie handling, no exe changes |

## External dependencies
- Rust 1.80+ toolchain, Node 20+, WebView2 runtime
- A Roblox OAuth app registration (client ID), for Phase 6
- A code-signing certificate, for Phase 8 release builds
- A Discord application ID, for Phase 7 RPC

## Phase 1 verification log (2026-11-16)
- `core` unit tests: 10 passed. Live client-version test passed (returned `0.747.0.7471481` / `version-4ea8e85481e34f46`).
- `cargo run --example status` on the dev PC detected the installed versions and the official handler, and reported "up to date".
- `npm run build` (tsc + vite) passes. The debug app built with the GNU toolchain and launched, and the shell rendered.
- Not yet verified: an MSVC build (Visual Studio Build Tools aren't installed on the dev PC).
