# Novastrap

A launcher and manager for the Roblox desktop client on Windows. Built with Tauri 2 (Rust) and React + TypeScript.

Novastrap is not affiliated with or endorsed by Roblox Corporation. It never modifies the Roblox executable, injects code, or handles your Roblox session cookie.

## Status
Working today: Roblox install detection, update check, **install/update through the official, signature-verified installer**, and **launching Roblox or joining a place by ID or link**. See [docs/FEATURES.md](docs/FEATURES.md) for the per-module checklist (implemented / partial / planned / not possible) and [docs/ROADMAP.md](docs/ROADMAP.md).

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
