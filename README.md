# Novastrap

A launcher and manager for the Roblox desktop client on Windows. Built with Tauri 2 (Rust) and React + TypeScript.

Novastrap is not affiliated with or endorsed by Roblox Corporation. It never modifies the Roblox executable, injects code, or handles your Roblox session cookie.

## Status
All 12 modules have a working backend and UI: launcher/installer, accounts (official OAuth), instances, discovery, servers, performance (allowlisted FastFlags), customization and client mods, catalog, trading values (Rolimons, attributed), second-screen HUD, Discord Rich Presence, and settings/storage/backups.
See [docs/FEATURES.md](docs/FEATURES.md) for exactly what is implemented, partial, or not possible without Roblox session cookies (which Novastrap never handles).

To use sign-in and Discord presence you need:
- A Roblox OAuth app (create.roblox.com → Credentials), with redirect `http://localhost:47821/oauth/callback` and scopes `openid profile`. Paste its client ID in Settings → Accounts.
- A Discord application ID (discord.com/developers). Paste it in Settings → Integrations.

## Develop
Prerequisites: Node 20+, Rust (MSVC toolchain), and **Visual Studio 2022 Build Tools with the "Desktop development with C++" workload**.

```bash
npm install
npm run tauri dev        # run the app
cargo test --manifest-path core/Cargo.toml   # backend unit tests
```

## Docs
- [Research on reference launchers](docs/RESEARCH.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap, risks and checklist](docs/ROADMAP.md)
- [Feature checklist](docs/FEATURES.md)

## License
MIT
