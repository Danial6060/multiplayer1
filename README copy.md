# Multiplayer FPS (MazeWars) — Minimal UDP Client/Server Skeleton

This repo is a Cargo workspace with two crates:

- `server/` — Headless Bevy + Renet UDP server (accepts 10+ clients; default max 32). Minimal networking only.
- `client/` — Two binaries:
  - CLI client (prompts for IP/username, sends Hello/Welcome)
  - Window client (default): opens a lightweight window with a simple grid and an FPS counter in the title

Scope is intentionally minimal so another developer can build the UI, gameplay, and tests on top.

## How to run (Windows PowerShell)

Open two terminals and run:

1) Start the server:

```powershell
cargo run -p server
```

The server binds to `0.0.0.0:5000` and prints your local IP. It uses UDP via Renet/Netcode.

2) Start the client (window, default):

```powershell
cargo run -p client
```

- Enter the server address (e.g., `127.0.0.1:5000`) and a username in the console.
- The client connects (sends Hello, expects Welcome) then opens a small window showing a grid and FPS in the window title.

To run the CLI client instead:

```powershell
cargo run -p client --bin client_cli
```

You can run multiple clients, including from other machines on the same network by pointing them to the server's LAN IP.

## Quick tips: server address and connection

- Use a numeric IP with port (IPv4): `127.0.0.1:5000` or e.g. `192.168.1.23:5000`.
- IPv6 needs brackets around the address: `[::1]:5000`, `[fe80::abcd]:5000`.
- Don’t omit the port: `127.0.0.1` is invalid; it must be `127.0.0.1:5000`.
- Don’t use `0.0.0.0:5000` as a target; that’s a bind address. Use the server’s local IP printed on startup (e.g., `192.168.x.y:5000`).
- Hostnames like `localhost:5000` aren’t accepted by the client right now; use the numeric IP `127.0.0.1:5000` instead.
- Same-machine testing: `127.0.0.1:5000`.
- Cross-machine testing: start the server, read the printed local IP (e.g., `192.168.x.y`), and use `192.168.x.y:5000` on other clients.

## What’s implemented

- UDP transport via `renet` + `renet_netcode` (client and server)
- Server accepts many clients (default 32; requirement minimum 10)
- Client prompts for server IP and username, then performs a Hello/Welcome handshake

## Where to extend (entry points)

Below are the places to implement the remaining features. Files are referenced by path.

### Client UI (mini-map, Maze-Wars style graphics, FPS counter)

- Introduce a GUI client (new crate or extend `client/`) using Bevy:
  - Add a Start/Exit screen, in-game HUD with a top-down minimap, and an FPS counter (`FrameTimeDiagnosticsPlugin`).
  - Add systems to send movement/rotation to server and receive updates.

### Level progression and round management

- Server side:
  - Gameplay modules were removed to keep this minimal. When ready, create new modules (e.g., `types`, `map`, `player`, `client`) and wire them in `server/src/lib.rs`.
  - Restore dependencies in `server/Cargo.toml` (e.g., `bevy_math`, `glam`, `rand`) as you reintroduce systems.
  - Implement a simple state machine (Lobby -> Countdown -> InRound -> RoundEnd) and broadcast transitions to clients.
- Client side:
  - Listen for server round messages; switch visuals/UI based on round state; show a countdown overlay.

### Tests and performance validation

- Add tests under both crates:
  - Server:
    - Unit-test `map::loader` and `client::movement::apply_wall_constraints`.
    - Property tests for map connectivity (no isolated rooms) using `proptest`.
  - Client:
    - Lightweight systems tests (e.g., FPS UI formatting) using `bevy`’s `World` to run systems headlessly.
- Simple perf checks:
  - Add an opt-in benchmark that spawns N dummy players and runs the server tick for X ms; ensure >50 FPS target on client render path.

### CI / build verification

- Add GitHub Actions workflow to build on Windows/Linux and run tests (see `docs/next_steps.md`).
- Cache cargo and set up a matrix (stable Rust, Windows-latest, ubuntu-latest).

## Minimal protocol

- Client sends `{ event: "Hello", body: { name } }` once when starting up.
- Server replies with `{ event: "Welcome", body: { message, client_id } }` — for connectivity confirmation.

## Notes

- Networking uses UDP via Renet’s netcode transport. Protocol ID is `0` for now; switch to a non-zero constant for production.
- The server runs headless using Bevy’s `ScheduleRunnerPlugin`. The client is CLI-only for now.
