# Agent Notes - Submarine Game

## Verify And Run

- Use `make check-all` for normal verification: native-check `shared`, `simulation`, and `server`, then check `client` for `wasm32-unknown-unknown`.
- Do not use a native client check as the main signal; Bevy's native audio stack may require system ALSA development libraries, while the supported client target is WASM/WebGL2.
- `make test` covers only `simulation` and `server`; client unit tests exist but have no supported runner in the Makefile. Run one native test with `cargo test -p <package> <test_name>` (for example, `cargo test -p simulation heading_updates`).
- Development requires two long-running processes: `make server` on `0.0.0.0:3000` and `make client` on `127.0.0.1:8080`.
- Build the deployable client with `make build-wasm`; Trunk writes to `crates/client/dist/`.
- `rust-toolchain.toml` installs stable Rust plus native Linux and `wasm32-unknown-unknown`. Bevy 0.19 requires Rust 1.95 or newer.
- Trunk 0.20.3 is known to work. If installing current Trunk from source fails in `lightningcss`, use `cargo install trunk --version 0.20.3 --locked`.

## Crate Boundaries

- `shared`: postcard wire protocol and serializable state; keep it limited to `serde`, `postcard`, and lightweight primitive/container types.
- `simulation`: deterministic game rules; it depends only on `shared`. Never add async, Bevy, Axum, or server state here.
- `server`: Axum WebSocket endpoint and lobby; each running game loop is a Tokio task receiving sequential commands through `mpsc`.
- `client`: Bevy 0.19 browser client. It depends on `shared`, not `simulation`; keep rendering compatible with WebGL2 rather than WebGPU-only or compute features.

## Wire And Runtime Details

- WebSocket frames are binary postcard values. The wire envelopes are `ClientMessage` and `ServerMessage`, not bare `PlayerCommand` and `GameEvent`; use `shared::codec::{encode, decode}`.
- `SubmarineState` keeps horizontal `x`/`y` separate from `depth`; depth has different physical constraints and must not be collapsed into a generic xyz vector.
- `crates/server/src/lobby.rs` owns role assignment and socket relay. `crates/server/src/game_room.rs` owns the 20 Hz authoritative simulation loop; keep those concerns separate.
- `ewebsock` uses non-`Send` WASM handles. `WsConnection` must remain a Bevy `NonSend` resource and be polled on the main thread.
- The client connects only after create/join and role selection. WASM derives `/ws` from the page origin, using port 3000 when Trunk runs on 8080; `?role=pilot` creates a room directly for development.
- Rendering interpolates `x`, `y`, and heading over the 50 ms snapshot interval; `GameState::submarine` remains the latest authoritative state used for commands.

## Design Sources

- Start at `docs/README.md`: `game-design.md` defines intended rules, `architecture.md` separates current and target architecture, and `roadmap.md` orders work as vertical slices.
- Treat target features in `docs/` as design, not implemented behavior. The executable code and manifests remain the source of truth for current behavior.
- Keep gameplay rules deterministic and server-authoritative. Update the GDD for rule changes, architecture docs for boundary/protocol changes, and the roadmap for scope or priority changes.

## Current Limitations

- Rooms are held in one in-memory registry, start explicitly with one to five humans, and fill free roles with bots. There is still no persistence or reconnection flow.
- Role-based command validation belongs in `game_room.rs`; rejected commands must not reach `Simulation` and the error must only be sent to the originating role.
- `Simulation::tick` applies basic nautical movement but produces no physics events. There are no collisions, inertia, or buoyancy yet.
- Sonar M3 is implemented with a private deterministic convoi, passive/active observations, uncertain tracks, role-filtered projections, and a tactile station. Repair still emits an event without mutating system state, and firing emits an event without creating a torpedo.
- The README calls the client a PWA, but there is no manifest or service worker yet. The hard-coded localhost WebSocket also prevents normal use from a remote phone.
- There is no CI, formatter/linter config, Dockerfile, or deployment config; do not infer those workflows from the roadmap.
