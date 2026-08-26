# Agent Notes - Submarine Game

## Verify And Run

- Use `make check-all` for normal verification: native-check `shared`, `simulation`, and `server`, then check `client` for `wasm32-unknown-unknown`.
- Do not use a native client check as the main signal; Bevy's native audio stack may require system ALSA development libraries, while the supported client target is WASM/WebGL2.
- Run native unit tests with `make test`; run one test with `cargo test -p <package> <test_name>` (for example, `cargo test -p simulation heading_updates`).
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
- The client connects to `ws://127.0.0.1:3000/ws`; select its role with `?role=pilot` (or another `CrewRole` name), defaulting to `Captain`.
- Rendering interpolates `x`, `y`, and heading over the 50 ms snapshot interval; `GameState::submarine` remains the latest authoritative state used for commands.

## Current Limitations

- The current lobby is one in-memory global lobby, starts when all five unique roles join, and has no persistence or reconnection flow.
- Role-based command validation belongs in `game_room.rs`; rejected commands must not reach `Simulation` and the error must only be sent to the originating role.
- `Simulation::tick` applies basic nautical movement but produces no physics events. There are no collisions, inertia, or buoyancy yet.
- There is no CI, formatter/linter config, Dockerfile, or deployment config in the repository yet; do not infer those workflows from the roadmap.
