# Next steps for full assignment

This guide points to exactly where to add features in this codebase.

## 1) Client UI and rendering

- Create modules under `client/src/`:
  - `ui/minimap.rs` — Top-down map. Read `server::map::types::Map` data replicated to clients (to be added) and draw the maze as lines/tiles. Show player dot.
  - `render/maze.rs` — 3D wall instances. Build meshes for `Wall` (x_wall / z_wall) from `server` messages.
  - `ui/fps.rs` — Add `FrameTimeDiagnosticsPlugin` and a text system to show FPS.
- Hook systems when state is `AppState::InGame`.
- Input: WASD + mouse. Serialize to JSON matching server handlers in `server/src/client/movement.rs` and `.../shooting.rs`.

## 2) Levels and round management

- Server `Game` state is in `server/src/types.rs`.
- Add a server-side round state enum and broadcast transitions.
- Drive map difficulty:
  - Use `Map::difficulty` and adjust the generator: `server/src/map/loader.rs -> remove_dead_ends`.
  - Provide 3 presets (Easy/Medium/Hard) and expose a vote or menu.

## 3) Replication

- Define message wrappers (e.g., `ServerMessage<T>`) and broadcast authoritative state snapshots each tick or on change.
- On client, consume messages to spawn/update player entities and the maze.

## 4) Tests and perf validation

- Server unit tests:
  - `map::loader` constraint: graph connectivity (no isolated rooms), dead-ends driven by difficulty.
  - `client::movement::apply_wall_constraints` edge cases.
- Client tests:
  - Systems tests for UI widgets (FPS text) using headless `bevy` world.
- Performance:
  - Add a criterion benchmark to simulate N players and measure server tick throughput.

## 5) CI / build verification

Create `.github/workflows/ci.yml`:

- Trigger on push/PR
- Cache Cargo
- Matrix: windows-latest, ubuntu-latest
- Steps: checkout, toolchain (stable), `cargo build --workspace --all-targets`, `cargo test --workspace`

## 6) Quality gates

- Lints: add `clippy` and `rustfmt` checks in CI.
- Add a `Makefile.toml` (cargo-make) or dev scripts for common tasks.
