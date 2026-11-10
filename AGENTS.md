# Agent Notes - Submarine Game

## Verify And Run

- Use `make check-all` for normal verification: native-check `shared`, `simulation`, and `server`, then check `client` for `wasm32-unknown-unknown`.
- Do not use a native client check as the main signal; Bevy's native audio stack may require system ALSA development libraries, while the supported client target is WASM/WebGL2.
- `make test` covers `shared`, `simulation`, and `server`, not client tests. Focus with `cargo test -p <package> <test_name>`; protocol fixtures alone: `cargo test -p shared fixtures`.
- Development requires two long-running processes: `make server` on `0.0.0.0:3000` and `make client` on `127.0.0.1:8080`.
- Build the deployable client with `make build-wasm`; Trunk writes to `crates/client/dist/`.
- `rust-toolchain.toml` selects stable plus native Linux and WASM targets; Trunk is a separate prerequisite. `crates/client/Trunk.toml` pins wasm-opt to `version_116`.

## Crate Boundaries

- `shared`: postcard wire protocol and serializable state; keep it limited to `serde`, `postcard`, and lightweight primitive/container types.
- `simulation`: deterministic game rules; it depends only on `shared`. Never add async, Bevy, Axum, or server state here.
- `server`: Axum WebSocket endpoint and lobby; each running game loop is a Tokio task receiving sequential commands through `mpsc`.
- `client`: Bevy 0.19 browser client. It depends on `shared`, not `simulation`; keep rendering compatible with WebGL2 rather than WebGPU-only or compute features.

## Wire And Runtime Details

- WebSocket frames are binary postcard values. The wire envelopes are `ClientMessage` and `ServerMessage`, not bare `PlayerCommand` and `GameEvent`; use `shared::codec::{encode, decode}`.
- `crates/shared/src/protocol.rs` owns `PROTOCOL_VERSION` and byte fixtures that catch enum reordering. Wire changes require coordinated client/server updates and fixture review; do not rely on the README's version number.
- `SubmarineState` keeps horizontal `x`/`y` separate from `depth`; depth has different physical constraints and must not be collapsed into a generic xyz vector.
- `crates/server/src/lobby.rs` owns role assignment and socket relay. `crates/server/src/game_room.rs` owns the 20 Hz authoritative simulation loop; keep those concerns separate.
- `ewebsock` uses non-`Send` WASM handles. `WsConnection` must remain a Bevy `NonSend` resource and be polled on the main thread.
- The client connects only after create/join and role selection. For solo development use `?role=captain`, then explicitly start the mission; a lone non-Captain cannot start. Vacant roles become bots, and Captain bot orders are rejected when that station has a human.
- WASM derives `/ws` from the page origin (matching `ws`/`wss`), substituting port 3000 for Trunk's 8080. `WS_URL` overrides only native builds. Trunk binds loopback by default, so phone testing needs an explicit externally reachable bind.
- Rendering interpolates `x`, `y`, and heading over the 50 ms snapshot interval; `GameState::submarine` remains the latest authoritative state used for commands.
- For touch-safe buttons, use `button_activation::enable` and consume `ButtonActivated` as in `periscope_ui.rs`: ordered pointer input handles taps completed within one frame and suppresses drags. Do not replace this with frame-sampled `Interaction::Pressed`.

## Design Sources

- Start at `docs/README.md`: `game-design.md` defines intended rules, `architecture.md` separates current and target architecture, and `roadmap.md` orders work as vertical slices.
- Treat target features in `docs/` as design, not implemented behavior. The executable code and manifests remain the source of truth for current behavior.
- Keep gameplay rules deterministic and server-authoritative. Update the GDD for rule changes, architecture docs for boundary/protocol changes, and the roadmap for scope or priority changes.

## Authority And Information Boundaries

- `game_room.rs` validates role permissions before calling `Simulation`; acknowledgments/errors go only to the originating player. It also owns role-specific snapshots and event filtering; preserve both when adding messages.
- Enemy truth stays private in `simulation`; never project real enemy identity, position, integrity or detection state. Captain/Weapons use shared estimates, Sonar owns acoustic observations, and only Captain receives periscope measurements and visual explosion events. Shared visual contacts are not automatic sonar tracks.
- `simulation/src/weapons.rs` owns tube procedures, solutions and torpedoes; `world.rs` owns private vessel integrity. An unobserved impact must not truncate the projected estimated torpedo run or confirm a kill.
- Tubes track ammunition (`loaded`) independently of shot settings. Keep local selection/edits distinct from applied settings; pending changes suspend firing. `WeaponsConfig::max_surface_impact_depth` governs both shot validation and collisions.
- Network events use a bounded 64-entry history. Explosion audio/effects also use separate counters/queues so bursts survive history eviction; do not reduce consumption to a single last event.
- Rooms are in-memory, with no persistence or reconnection. `main.rs` serves only `/ws`, not the built client; `infra/` is a placeholder, not a deployable setup.
