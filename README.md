# Novastrap

A launcher and manager for the Roblox desktop client on Windows. Built with Tauri 2 (Rust) and React + TypeScript.

Novastrap is not affiliated with or endorsed by Roblox Corporation. It never modifies the Roblox executable, injects code, or handles your Roblox session cookie.

## Status
Phase 1 (foundation): Roblox install detection, latest-version check, protocol handler detection, and the app shell. See [docs/ROADMAP.md](docs/ROADMAP.md).

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

## License
MIT
