# Feature completion checklist

Legend: ✅ implemented and tested · 🟡 partially implemented · ⬜ planned · ⛔ not possible within our rules (the reason and the alternative are given)

**The rules that decide feasibility:** no Roblox session cookie (`.ROBLOSECURITY`), no passwords, no code injection, no Hyperion interaction, no modifying the executable, and only supported or clearly identified data sources. Many Roblox web APIs (friends' presence, trades, avatar editing, purchases, inventory) **only work with the session cookie**. Roblox's official OAuth 2.0 gives a much narrower set of scopes. Where a feature needs the cookie, it's marked ⛔ and gets a cookie-free alternative.

Build order follows the requested priority. Each module is built only after checking what already exists.

---

## 1. Core launcher and installation
| Feature | Status | Notes |
|---|---|---|
| Detect installed versions | ✅ | `core/src/roblox/detect.rs` |
| Detect the protocol handler owner | ✅ | Official, Bloxstrap, Fishstrap, Novastrap, other |
| Latest-version check with fallback | ✅ | `deployment.rs`, validated response |
| Install / update via the official installer | ✅ | Downloads `RobloxPlayerInstaller.exe` from `setup.rbxcdn.com`. **Authenticode-verified (signer must be Roblox Corporation)** before it runs. Progress is shown in the UI |
| Interrupted-download recovery | ✅ | Downloads go to `.partial` files that are never executed. Leftovers are cleaned at the next install |
| Launch the Roblox app | ✅ | Runs the installed `RobloxPlayerBeta.exe` unmodified |
| Join an experience by place ID / link | ✅ | `roblox://experiences/start?placeId=…` passed to the client, the same way the registered handler does it |
| Join a specific server (job ID) | ✅ | `gameInstanceId`. The UI arrives with the Server Browser (module 5) |
| Opt-in protocol takeover | ⬜ | Phase 4b: register Novastrap as the handler, with a saved restore |
| Managed installs / version pinning | ⬜ | Bloxstrap-style manifest plus MD5 into our own folder (module 6) |

## 2. Account manager
| Feature | Status | Notes |
|---|---|---|
| Sign in with Roblox (OAuth 2.0 + PKCE) | ⬜ | Official flow. Needs a client ID registered at create.roblox.com |
| Profile: username, display name, avatar, ID | ⬜ | OAuth `openid profile` plus public users and thumbnails APIs |
| Multiple saved accounts and switching | ⬜ | Switches the **launcher** identity |
| Tokens in Windows Credential Manager | ⬜ | `keyring` crate. Never written to disk or logs |
| Switching the account the **Roblox client** uses | ⛔ | Needs the session cookie. Alternative: Novastrap shows which account the client last used (read from the client log) and links to Roblox's own account switcher |

## 3. Instance manager
| Feature | Status | Notes |
|---|---|---|
| Multiple simultaneous Roblox windows | ⬜ | Opt-in. Works by holding Roblox's single-instance mutex (the Fishstrap/Fedestrap technique). It doesn't touch the client, but Roblox doesn't officially support it, so the UI says so |
| Instance naming, status, start/close individual instances | ⬜ | Process tracking per launched PID |
| Saved instance profiles (FastFlags + launch target) | ⬜ | Profiles from module 6 |
| **Separate account per instance** | ⛔ | Each instance would need its own session cookie. All instances use the account the Roblox client is signed in with |
| Independent cache directories | ⛔ | The client hard-codes `%LOCALAPPDATA%\Roblox`. No supported override exists |

## 4. Game discovery
| Feature | Status | Notes |
|---|---|---|
| Search, trending, popular, categories | ⬜ | explore-api / search-api. Undocumented, so placed behind a validating adapter |
| Details, descriptions, thumbnails, player counts | ⬜ | games / thumbnails APIs (documented) |
| Favorites | ⬜ | Local favorites (writing to Roblox favorites needs the cookie ⛔) |
| Recommendations | ⬜ | Local, built from your play history and the genres of games you favorite. Roblox's personalized recommendations need the cookie |

## 5. Server browser
| Feature | Status | Notes |
|---|---|---|
| List public servers, players, capacity, pagination, refresh | ⬜ | `games.roblox.com/v1/games/{placeId}/servers/Public` |
| Join a selected server | ✅ (backend) | Launch target with `gameInstanceId` is done. The page is next |
| Favorite servers | ⬜ | Local, by job ID. Expires when the server closes |
| Ping | 🟡 | Roblox returns a `ping` field per server, measured from Roblox's side. It's shown as "reported ping" |
| Region before joining | ⛔ | Not in the API. Shown after joining, from the log IP, as an opt-in |
| Join friends | ⛔ | Friend presence with game IDs needs the cookie. Alternative: Roblox's own "Join" from the Roblox app |

## 6. Performance center
| Feature | Status | Notes |
|---|---|---|
| FPS cap, graphics quality, allowlisted FastFlags, presets, profiles | ⬜ | Writes `ClientSettings/ClientAppSettings.json` in the version folder |
| Memory monitoring | ⬜ | The Roblox process's working set, read through standard Windows process APIs (read-only) |
| Launch diagnostics | ⬜ | Install integrity, handler, version, last client log errors |
| Version pinning | ⬜ | Requires managed installs |

## 7. Mod and customization center
| Feature | Status | Notes |
|---|---|---|
| Launcher themes, backgrounds, fonts, sounds | ⬜ | Launcher-side only |
| Client cursors, fonts, sounds | ⬜ | Replaces files under the version's `content\` folder (Bloxstrap's mod model). Each changed file is backed up and reversible. Never the exe |
| Compatibility checks | ⬜ | Validated against the installed version's content tree |
| Import / export | ⬜ | Zip of the mods and profile JSON |

## 8. Catalog and avatar
| Feature | Status | Notes |
|---|---|---|
| Catalog search, item thumbnails, details, prices | ⬜ | `catalog.roblox.com` public search and details |
| Local favorite items | ⬜ | |
| Avatar preview (your current avatar) | ⬜ | Public avatar thumbnails |
| Avatar editor / try-on / purchase | ⛔ | Write endpoints need the cookie. Alternative: an "Open on Roblox" button for the item page |

## 9. Trading center
| Feature | Status | Notes |
|---|---|---|
| Item values from a third-party provider | ⬜ | Rolimons public item data. Every value is labelled "Source: Rolimons", with a timestamp |
| Item comparison | ⬜ | Side by side, values with their source |
| Trade history / inbound trades / notifications | ⛔ | Trade APIs need the cookie. Alternative: link to Roblox's own trades page |

## 10. Second-screen HUD
| Feature | Status | Notes |
|---|---|---|
| Separate always-on-top window | ⬜ | A second Tauri window |
| Current game, server, session duration | ⬜ | Taken from the client log watcher |
| Current account | ⬜ | The launcher account, or the user ID seen in the client log |
| Notifications | ⬜ | Launcher events |
| In-game FPS | ⛔ | Reading the client's frame rate means hooking the process. Alternative: Roblox's built-in Shift+F5 stats |
| Friends activity | ⛔ | Needs the cookie, same as above |

## 11. Discord integration
| Feature | Status | Notes |
|---|---|---|
| Rich Presence with the current game and server | ⬜ | Discord IPC plus the log watcher |
| Custom presence text, privacy toggles (hide game, hide join button) | ⬜ | |

## 12. Settings and storage
| Feature | Status | Notes |
|---|---|---|
| Settings store (versioned JSON, atomic writes) | ⬜ | |
| Pages: general, appearance, performance, accounts, instances, integrations, notifications | ⬜ | Each one arrives with its module |
| Cache management and storage usage | ⬜ | Novastrap's cache, downloads, logs, old Roblox version folders |
| App updates | ⬜ | tauri-plugin-updater with signed releases |
| Logs | 🟡 | Rotating file logs are written. A viewer page is planned |
| Configuration backups | ⬜ | |

## Home dashboard (cross-cutting)
| Feature | Status | Notes |
|---|---|---|
| Install status, update, launch, join by link | ✅ | |
| Recently played, playtime | ⬜ | Local history from the log watcher (module 11) |
| Favorites, recently used accounts | ⬜ | After modules 2 and 4 |
| Friends activity | ⛔ | Needs the cookie |
| Custom backgrounds | ⬜ | Module 7 |
