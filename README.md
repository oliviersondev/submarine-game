# Submarine Game

A cooperative multiplayer 2D submarine game built in Rust. Each player takes a crew role aboard the same submarine — captain, pilot, sonar operator, engineer, or weapons officer — and must cooperate in real time to complete missions.

Playable directly in the browser (desktop and mobile) as a Progressive Web App, with no installation required.

## Tech stack

| Layer | Technology |
|---|---|
| Client | Bevy 2D compiled to WebAssembly |
| Web / Mobile | Progressive Web App (PWA) |
| Server | Rust + Tokio + Axum |
| Real-time transport | Secure WebSocket (WSS) |
| Wire format | Binary — `serde` + `postcard` |
| Simulation | Deterministic Rust crate, server-side only |
| Persistence | PostgreSQL (optional — accounts and progression) |

## Crew roles

| Role | Responsibilities |
|---|---|
| Captain | Tactical map, mission orders, crew coordination |
| Pilot | Depth, heading, speed |
| Sonar | Detection, contact identification and tracking |
| Engineer | Power management, damage control, repairs |
| Weapons | Torpedo loading, tube management, targeting |

Each role receives only the information relevant to its station. The sonar operator sees raw contacts; the captain sees the tactical picture once contacts are classified and shared.

## Architecture

```
Browser / PWA (Bevy + WASM)
        |
        | WSS — PlayerCommand / GameEvent (binary)
        |
Rust server (Axum + Tokio)
        |
        +-- Lobby & matchmaking (in memory)
        +-- Game rooms (one Tokio task per room)
        +-- Authoritative simulation (10–20 ticks/s)
        |
        +-- PostgreSQL (optional)
```

The server is authoritative. Clients send intents (`SetEnginePower`, `LoadTorpedo`, `RepairSystem`, …); the server validates them against the player's assigned role, runs the simulation, and broadcasts state events to all crew members.

## Workspace layout

```
submarine-game/
├── Cargo.toml          # workspace
├── crates/
│   ├── shared/         # commands, events, IDs, wire protocol
│   ├── simulation/     # deterministic submarine rules (no Bevy, no Axum)
│   ├── server/         # Axum, WebSocket, lobby, game rooms
│   └── client/         # Bevy 2D, station UIs, rendering
├── assets/
│   ├── sprites/
│   ├── audio/
│   └── fonts/
└── infra/              # Dockerfile, deployment scripts
```

The `simulation` crate has no dependency on Bevy or Axum and can be unit-tested independently.

## Prerequisites

- Rust (stable, latest) — https://rustup.rs
- `wasm-pack` — `cargo install wasm-pack`
- `trunk` (Bevy WASM bundler) — `cargo install trunk`
- PostgreSQL (optional, only for persistence features)

## Getting started

```bash
# Clone the repository
git clone <repo-url>
cd submarine-game

# Run the server (development)
cargo run -p server

# Run the client in the browser (hot-reload)
cd crates/client
trunk serve
# Open http://localhost:8080
```

## Network protocol

Clients send `PlayerCommand` messages; the server emits `GameEvent` messages. Both are serialised with `postcard` over a binary WebSocket channel.

```rust
// crates/shared/src/protocol.rs
enum PlayerCommand {
    SetEnginePower(u8),
    ChangeDepth(i16),
    RotateSonar(f32),
    LoadTorpedo { tube: u8 },
    RepairSystem { system: SystemId },
}

enum GameEvent {
    SubmarineStateChanged(SubmarineState),
    SonarContactDetected(Contact),
    SystemDamaged(SystemId),
    TorpedoLoaded(u8),
}
```

## Deployment

Initial target: a single server (VM or ECS Fargate container) behind an Application Load Balancer with HTTPS/WSS termination.

Horizontal scaling (multiple game server instances) is deferred until vertical capacity is genuinely exhausted. It requires routing each reconnecting client to the correct server instance.

## Roadmap

- [ ] Workspace skeleton and shared protocol crate
- [ ] Server: WebSocket lobby and room management
- [ ] Simulation: submarine physics and systems
- [ ] Client: Bevy 2D base and WebSocket integration
- [ ] Station UIs: pilot, sonar, engineer, weapons, captain
- [ ] PWA manifest and offline splash screen
- [ ] Mission scenarios
- [ ] Accounts and progression (PostgreSQL)
- [ ] Containerisation and CI/CD
