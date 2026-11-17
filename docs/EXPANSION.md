# Expansion audit (2026-11-16)

This is the audit of the proposed features against what Novastrap already has, done before writing any code. The status column is updated as stages land. [FEATURES.md](FEATURES.md) remains the main checklist.

Legend:
- ✅ already exists
- 🟡 partly exists
- ➕ missing and realistic (planned in a stage below)
- 🔑 needs an external API, credentials or release infrastructure
- ⛔ not supported or not realistic under our rules

## 1. Launcher experience
| Feature | Before | Notes |
|---|---|---|
| System tray | ➕ | Tauri's built-in tray. Menu: Show, Play, HUD, Quit. Optional close-to-tray |
| Global keyboard shortcuts | ➕ | Official Tauri global-shortcut plugin. Customizable, off by default, and validated |
| Notification center | 🟡 | Toasts and two OS notifications existed, but nothing was kept. Add a persisted in-app log |
| Stable / beta update channels | 🟡 🔑 | An update check existed (GitHub "latest release"). Beta = include GitHub pre-releases. Needs published releases on Ahere12/Novastrap, and there are none yet |
| Changelog and update history | ➕ 🔑 | Changelog = GitHub release notes (needs releases). Update history = versions this PC has run, recorded locally |
| Rollback of Novastrap updates | ⛔ for now 🔑 | Needs a self-updater and signed release builds. Neither exists, so there is nothing to roll back. Until then, the release list links to earlier releases |

## 2. Roblox management
| Feature | Before | Notes |
|---|---|---|
| Repair the Roblox install | 🟡 | Install and update existed. Add a check (executable present, Roblox Corporation signature, protocol handler) plus "Repair", which re-runs the official signed installer |
| Roblox version management | ⛔ | The official installer only installs the current version. Pinning would need Bloxstrap-style manual package downloads from Roblox's CDN, outside the official path. We keep: showing installed versions and removing old ones (already in Storage) |
| Saved launch profiles | ✅ | Instance profiles |
| Favorite game shortcuts | 🟡 | Favorites existed. Add "Desktop shortcut", a `.url` file using Roblox's official `roblox://` link |
| Recently played | ✅ | Home |
| Server history | 🟡 | Sessions already stored the server (job) ID. Add a list with Rejoin. Rejoin only works while that server is still running |
| Configurable storage location | ➕ (partial) | Large data (downloads, mods, backgrounds) can move to a chosen folder. Settings themselves stay in AppData, since something has to say where the rest lives |

## 3. Customization
| Feature | Before | Notes |
|---|---|---|
| Custom accent colors | ✅ | |
| Sidebar customization and reordering | ➕ | Reorder and hide sections (Play and Settings can't be hidden) |
| Custom backgrounds | ✅ | |
| Compact mode | ➕ | Denser spacing |
| Reduced motion | ✅ | |
| More theme customization | 🟡 | 3 themes + accent + font. Add text size and compact mode |

## 4. Advanced tools
| Feature | Before | Notes |
|---|---|---|
| Diagnostics center | 🟡 | Launch diagnostics existed (Performance → Diagnostics). Add the repair check and the app log next to it |
| Safe mode | ➕ | Launch Roblox once with no FastFlags and no mods. Mods are restored and flags re-written on the next normal launch |
| Advanced backup and restore | 🟡 | Export/import existed (all settings, no tokens). Add choosing which parts to restore |
| Storage analyzer | 🟡 | Usage per category + clearing existed. Kept as is; the new storage location feeds into it |
| Download manager | 🟡 | Novastrap only downloads one thing: the official Roblox installer, with progress, verification and resume cleanup. A general download manager would have nothing to manage. Kept as is |
| Import/export launcher settings | ✅ | Settings → Storage & backups |
| Accessibility | 🟡 | Focus outlines, labels and reduced motion existed. Add text size |
| Multiple languages | ➕ (deferred) | Every string is hard-coded English. Translating needs an i18n layer across all pages, plus translations reviewed by native speakers. Deferred: machine-written translations shipped untested would break the "don't pretend" rule |

## Needs external APIs or credentials
- **Update channels, changelog, rollback:** published GitHub releases on `Ahere12/Novastrap`. Reading them is public, with no token. Rollback also needs signed builds and an updater key.
- **Unchanged from before:** a Roblox OAuth client ID (sign-in) and a Discord application ID (Rich Presence).
- Nothing here uses Roblox cookies, session tokens or bypasses.

## Implementation order
1. Tray, close-to-tray, global shortcuts
2. Notification center
3. Server history, desktop game shortcuts
4. Safe-mode launch, repair check
5. Compact mode, sidebar order and visibility, text size
6. Update channels, changelog, local update history
7. Later: storage location, selective restore, languages

## Results (2026-11-16)

How each item was tested:
- **live**: exercised in the running app or against the real service
- **unit**: covered by automated tests on the core logic
- **built**: compiles and renders, but the action itself wasn't run

| Feature | Status | Tested |
|---|---|---|
| System tray (Open, Play, HUD, Quit) and close-to-tray | ✅ | live: closing with the option on hides the window and keeps the app running; the tray icon brings it back |
| Global shortcuts (open, HUD, play), customizable, off by default | ✅ | live: Ctrl+Alt+N showed the hidden window; Ctrl+Alt+H opened then closed the HUD. unit: parsing and duplicate rejection. The Play shortcut uses the same launch path as the Play button but wasn't pressed, to avoid starting Roblox during the test |
| Notification center (bell, unread count, clear) | ✅ | Records Roblox updates, game joins, safe-mode launches and launch problems from the tray or shortcuts. unit: log ordering and cap. built: the bell renders; entries appear once something happens |
| Stable/beta channel, changelog, update history | ✅ 🔑 | unit: channel filtering, version ordering, rejecting non-GitHub links. live: the update history recorded 0.1.0 on first run. Changelog shows "no releases yet" until releases are published |
| Rollback | ⛔ for now | Needs a self-updater and signed releases. The changelog explains this and links each release page |
| Repair check and repair | ✅ | live: Diagnostics shows the player's Authenticode result (this PC: Valid, Roblox Corporation). Repair re-runs the already-tested official installer flow |
| Safe-mode launch | ✅ built | No FastFlags and Roblox's original files for one launch; restored on the next normal launch. Not run, to avoid starting Roblox during testing |
| Server history with Rejoin | ✅ built | From local history; one row per server |
| Desktop game shortcut | ✅ unit | `.url` file with the `roblox://experiences/start?placeId=` link; name cleaned up for Windows. The button wasn't clicked in the app |
| Compact mode, text/interface size, sidebar reorder/hide | ✅ built | unit: settings validation. Play can't be hidden; Settings stays last |
| Discord Rich Presence | ✅ live | With the app ID 1572124061524434985: Discord accepted the connection; a test presence was set and cleared (`cargo run --example discord_test -- <id>`) |
| Configurable storage location, selective restore, languages | ⬜ | Deferred (see above) |
