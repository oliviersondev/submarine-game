# Agent Notes — Submarine Game

## Project overview

Cooperative 2D multiplayer submarine game in Rust. Players take crew roles (captain, pilot, sonar, engineer, weapons) and connect via WebSocket to a shared authoritative simulation. The client compiles to WebAssembly and is served as a Progressive Web App.

## Workspace layout

```
submarine-game/
├── Cargo.toml              # workspace — members = ["crates/*"]
├── crates/
│   ├── shared/             # wire protocol: PlayerCommand, GameEvent, SystemId, enums
│   ├── simulation/         # deterministic submarine rules — NO Bevy, NO Axum, NO async
│   ├── server/             # Axum + Tokio: HTTP, WebSocket, lobby, game rooms
│   └── client/             # Bevy 2D + WASM: rendering, station UIs, input
├── assets/                 # sprites, audio, fonts — loaded by Bevy AssetServer
└── infra/                  # Dockerfile, deployment configs
```

## Crate responsibilities and dependency rules

- `shared` — zero heavy dependencies; only `serde`, `postcard`, and primitive types. Both server and client depend on it.
- `simulation` — pure deterministic logic. No async, no Bevy, no Axum. Depends only on `shared`. Must be unit-testable with plain `cargo test`.
- `server` — depends on `shared` and `simulation`. Uses Tokio async runtime and Axum for HTTP + WebSocket. Holds game rooms as Tokio tasks with `mpsc` channels.
- `client` — Bevy 2D compiled to WASM. Depends on `shared`. Communicates with the server via a WebSocket connection managed outside Bevy's ECS (e.g. `web_sys` or `ewebsock`).

Never add Bevy or Axum as a dependency of `simulation`. Never add async code to `simulation`.

## Network protocol

All messages are serialised with `postcard` (binary) over WebSocket.

- Client → Server: `PlayerCommand` (defined in `shared`)
- Server → Client: `GameEvent` (defined in `shared`)

The server validates each command against the sender's assigned crew role before applying it to the simulation. A reconnecting client receives a full `SubmarineState` snapshot.

## Simulation tick rate

10–20 ticks per second server-side. The client interpolates between received states to render at 60 fps. Do not increase tick rate without measuring actual bandwidth and CPU impact.

## Crew roles

`captain`, `pilot`, `sonar`, `engineer`, `weapons`. Defined in `shared::protocol::CrewRole`. The server filters `GameEvent` per role — sonar operators receive raw contacts, not classified ones.

## Bevy and WebGL2

The client targets WebAssembly + WebGL2. Keep render features within `Limits::downlevel_webgl2_defaults()`. Do not enable features that require WebGPU or compute shaders. Test on Safari iOS and Android Chrome before declaring any visual feature done.

## Development commands

```bash
# Server (native)
cargo run -p server

# Client (browser, hot-reload via trunk)
cd crates/client && trunk serve

# Run all tests (simulation crate is the main test target)
cargo test -p simulation

# Build WASM release
cd crates/client && trunk build --release
```

## Key conventions

- Game state lives in memory on the server. No Redis, no database in the hot path.
- One Tokio task per game room. Commands are processed sequentially inside the task; avoid shared locks on game state.
- The lobby is a separate concern from the game room. Keep them in distinct modules under `server`.
- Assets are loaded via Bevy's `AssetServer`. Keep asset paths relative to the `assets/` root.
- Do not introduce microservices, Kubernetes, or a message broker until vertical scaling is genuinely exhausted.

## Out of scope (for now)

- 3D rendering — 2D only
- Native iOS / Android apps — PWA covers mobile
- Integrated voice chat — use an external solution
- Multi-region deployment
- WebRTC / UDP transport
- Rollback netcode (GGRS) — not needed for cooperative turn-paced crew roles

## Deployment target

Single container (Rust binary) behind an AWS Application Load Balancer with HTTPS/WSS termination, targeting ECS Fargate. The ALB supports WebSocket upgrades natively — no special configuration needed beyond a standard HTTP listener.
