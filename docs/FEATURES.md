# Feature completion checklist

Legend:
- ✅ **implemented**: backend tested (unit tests, plus a live test against the real service where noted), and the UI is built
- 🟡 **partial**: works, with a stated gap
- ⬜ **planned**
- ⛔ **not possible within our rules** (the reason and the alternative are given)

**The rules that decide feasibility:** no Roblox session cookie, no passwords, no injection, no Hyperion interaction, no modifying executables, and only supported or clearly attributed data. Many Roblox web APIs (friends, presence, trades, avatar editing, purchases) only work with the session cookie, so features needing them are marked ⛔ and get the alternative shown.

**Layout (2026-11-16):** a game-launcher layout. A rail on the left holds the account chip and four sections (Roblox Player, Explore, Marketplace, Tuning), with HUD and Settings at the bottom. Each section has its own row of tabs across the top. The Play tab is a banner (a screenshot of the last-played experience, or original artwork) above a Play bar with an instance picker, a large Play/Install button and the version/update status. All artwork and styling are original. The Play-bar instance choice is saved as `instances.default_profile`.

**Themes:** two styles under Customize → Theme. **Classic** (the default) has an early-2000s web look: a light page, a blue header strip with folder tabs, glossy gradient buttons, panels with title bars, Verdana text and yellow tooltip-style toasts. **Launcher** is the dark look. Both use original styling; no Roblox logos, wordmarks or artwork are used. Saved settings with the old internal theme name `nova` (never selectable) display as Classic.

**UI verification status (2026-11-16):** every page compiles under strict TypeScript and the app runs. The shell, navigation, Accounts and the Discover loading state were checked in the running app. The other pages haven't been looked at on screen yet: testing stopped when the developer started playing Roblox. Network commands now log their timing to the app log for the next check.

Tests: `core` has **68 unit tests** plus **10 live tests** (`cargo test -- --ignored`) against Roblox, Rolimons, Windows Credential Manager and real client logs.

---

## 1. Core launcher and installation
| Feature | Status | Notes |
|---|---|---|
| Detect installs, handler, latest version | ✅ live | |
| Install/update via the official signed installer | ✅ live | The download and Roblox Corporation signature check were tested live. The installer itself wasn't run during testing (Roblox was already up to date) |
| Launch the app; join by place ID, link or server | ✅ live | The app launch was tested live. Joining a place/server uses the same argument format as Roblox's protocol handler |
| Applies the active performance profile and mods before each launch | ✅ | |
| Interrupted-download recovery | ✅ | |
| Startup notification when a Roblox update is available | ✅ | |
| Opt-in protocol takeover | ⬜ | |

## 2. Account manager
| Feature | Status | Notes |
|---|---|---|
| Sign in with Roblox (OAuth 2.0 + PKCE, loopback redirect) | 🟡 | Fully implemented and unit-tested, including a loopback round trip. **Not run end-to-end**: it needs your OAuth client ID from create.roblox.com |
| Tokens in Windows Credential Manager, refresh, revoke on removal | ✅ live | Credential Manager round trip tested live |
| Saved profiles by username (public data), avatar, display name, ID | ✅ live | |
| Multiple accounts, switching, recently used, session status | ✅ | Status is Signed in / Session expired / Profile only |
| Switching the account the **Roblox client** uses | ⛔ | Needs the session cookie. The client keeps its own login |

## 3. Instance manager
| Feature | Status | Notes |
|---|---|---|
| Saved instance profiles (name, launch target, performance profile) | ✅ | |
| Start an instance; status, memory, uptime; close a client | ✅ | Closing refuses any process that isn't `RobloxPlayerBeta.exe` |
| Several clients at once | ⛔ (tested) | Both known techniques (holding `ROBLOX_singletonMutex`, and squatting `ROBLOX_singletonEvent` with a mutex) were tried live on client 0.747. The second client handed off to the first both times, so the code was removed rather than shipped broken |
| Separate account per instance | ⛔ | Needs a session cookie per instance |
| Independent cache directories | ⛔ | The client hard-codes `%LOCALAPPDATA%\Roblox` |

## 4. Game discovery
| Feature | Status | Notes |
|---|---|---|
| Trending, popular and other Roblox home sorts | ✅ live | explore-api (undocumented, strictly validated). Sponsored entries are removed |
| Search | ✅ live | omni-search |
| Categories | ✅ | Genre filter built from the genres Roblox returns |
| Detail page: description, screenshots, icon, players, visits, rating, favorites, updated date | ✅ live | |
| Favorites | ✅ | Local |
| Recommendations | ✅ | Local, by genre affinity from your favorites and play history. It says so when there's no data yet |

## 5. Server browser
| Feature | Status | Notes |
|---|---|---|
| Public servers: players, capacity, server FPS, ping, sorting, pagination, refresh | ✅ live | Ping is labelled "reported by Roblox" (it's server-side, not your own latency) |
| Join a selected server | ✅ | |
| Favorite servers | ✅ | Local. Servers expire when empty |
| Region before joining | ⛔ | Not provided by Roblox's API |
| Join friends | ⛔ | Presence needs the cookie |

## 6. Performance center
| Feature | Status | Notes |
|---|---|---|
| Allowlisted FastFlag editor (17 flags, typed and range-checked) | ✅ | Non-allowlisted flags are rejected and reported (Roblox ignores them since Sept 2025) |
| Frame-rate cap and graphics quality (Roblox's own settings file) | ✅ | Refused while Roblox is running (it would overwrite them). One-time backup of the file |
| Presets, custom profiles, active profile, import (Novastrap or Bloxstrap JSON) and export | ✅ | |
| Memory monitoring | ✅ | Per client, on Instances and Diagnostics |
| Launch diagnostics (install, handler, flags file, settings, mods, recent client log errors) | ✅ | |
| FPS above 240 | ⛔ | Roblox doesn't support it, and the FPS flag isn't allowlisted |
| Version pinning | ⬜ | Needs a managed (non-official-installer) install |

## 7. Mod and customization center
| Feature | Status | Notes |
|---|---|---|
| Client mods: arrow, far-arrow and text cursors, death sound | ✅ | File signature, size and target-exists checks. Originals backed up; revert restores them. Re-applied after updates |
| Launcher accent color, font, background image with dimming, reduced motion, UI sounds | ✅ | |
| Theme import/export | ✅ | |
| Client font replacement | ⬜ | |

## 8. Catalog and avatar
| Feature | Status | Notes |
|---|---|---|
| Search, categories, sorting, pagination, thumbnails, prices, stock, details | ✅ live | |
| Favorite items | ✅ | Local |
| Avatar preview of the active account | ✅ live | |
| Avatar editor, try-on, purchases | ⛔ | Needs the cookie. "Open on Roblox" / "Edit on Roblox" instead |

## 9. Trading center
| Feature | Status | Notes |
|---|---|---|
| Item values (value, RAP, demand, trend, projected/rare) | ✅ live | Source shown on the page: Rolimons, with fetch time. Unvalued items say "Not valued", never 0 |
| Trade comparison calculator | ✅ | |
| Trade history, offers, notifications | ⛔ | Needs the cookie |

## 10. Second-screen HUD
| Feature | Status | Notes |
|---|---|---|
| Always-on-top window: game, account, session time, server, Discord status, recent events | 🟡 | The first build rendered blank. Fixed (window marker via init script) but not yet re-checked on screen |
| In-game FPS | ⛔ | Would need hooking the client. Points to Roblox's Shift+F5 |
| Friends activity | ⛔ | Needs the cookie |

## 11. Discord integration
| Feature | Status | Notes |
|---|---|---|
| Rich Presence: game, icon, elapsed time, optional join button, account, custom text | 🟡 | Implemented and privacy logic unit-tested. Needs your Discord application ID to run, so not tested end-to-end |
| Privacy toggles (hide game, join button off by default, account hidden by default) | ✅ | |

## 12. Settings and storage
| Feature | Status | Notes |
|---|---|---|
| General, accounts, integrations, notifications, privacy | ✅ | Validated before saving |
| Storage usage; clear downloads and cache; remove old Roblox versions (blocked while Roblox runs) | ✅ | |
| Logs viewer, open log folders | ✅ | |
| Backups: export/import (never includes tokens) | ✅ | |
| App update check (GitHub releases) | ✅ | Self-updating install is ⬜ (needs signed releases) |

## Home (cross-cutting)
| Feature | Status | Notes |
|---|---|---|
| Live "in game" card, recently played, playtime (7 days / total / top games), Rejoin | ✅ | Activity watcher (log replay tested on a real client log) |
| Favorites, recent accounts, install/update/play/join | ✅ | |
| Friends activity | ⛔ | Needs the cookie |
| Custom background | ✅ | |
