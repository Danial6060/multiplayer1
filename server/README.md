Server

This crate is now a minimal UDP server skeleton for the multiplayer FPS project. It provides only:

- UDP transport over Renet/Netcode
- Connect/disconnect logging
- A simple Hello/Welcome JSON handshake

All gameplay systems (map generation, player movement, shooting, timers, votes) have been removed to keep this crate minimal. Reintroduce them later as needed.

## Top-level files

- `Cargo.toml` — Rust crate manifest and dependency list (trimmed to networking + Bevy app/time + serde).
- `README.md` — (this file) overview of the current minimal scope.

## Source (`src/`)

- `src/lib.rs` — server library entry; sets up Renet server and handles Hello/Welcome and connection logs.
- `src/main.rs` — tiny binary that calls `run_server` with defaults.

To add gameplay later, create modules (e.g., `types`, `client`, `player`, `map`) and wire systems back into `src/lib.rs`, restoring any needed dependencies in `Cargo.toml`.

