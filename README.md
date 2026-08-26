# Submarine Game

Jeu coopératif multijoueur 2D de sous-marin en Rust. Les joueurs incarnent différents membres d'équipage et se connectent via WebSocket à une simulation autoritaire côté serveur. Le client compile en WebAssembly et est servi comme Progressive Web App.

## Vue d'ensemble

Jusqu'à 5 joueurs rejoignent une même salle, chacun avec un rôle distinct :

| Rôle | Responsabilité |
|------|----------------|
| `Captain` | Coordination générale, décisions tactiques |
| `Pilot` | Cap, vitesse, profondeur |
| `Sonar` | Détection des contacts (contacts bruts non classifiés) |
| `Engineer` | Réparation des systèmes, gestion de l'énergie |
| `Weapons` | Tir des torpilles |

La partie démarre automatiquement quand les 5 rôles sont occupés.

## Architecture

```
submarine-game/
├── Cargo.toml              # workspace Cargo (resolver = "2")
├── rust-toolchain.toml     # stable + wasm32-unknown-unknown
├── Makefile                # commandes de développement
├── assets/                 # sprites, audio, polices (AssetServer Bevy)
├── infra/                  # Dockerfile, configs déploiement
└── crates/
    ├── shared/             # protocole réseau — serde + postcard
    │   └── src/
    │       ├── lib.rs      # re-exports publics
    │       ├── state.rs    # CrewRole, SystemId, SystemStatus, SubmarineState
    │       ├── protocol.rs # ClientMessage, ServerMessage, PlayerCommand, GameEvent, ProtocolError
    │       └── codec.rs    # encode() / decode() postcard
    ├── simulation/         # logique déterministe — NO async, NO Bevy
    │   └── src/lib.rs      # Simulation::new(), tick(dt), apply_command()
    ├── server/             # Axum 0.8 + Tokio — binaire natif
    │   └── src/
    │       ├── main.rs     # écoute 0.0.0.0:3000, route /ws
    │       ├── lobby.rs    # LobbyState, attribution des rôles, ws_handler
    │       └── game_room.rs# boucle de jeu 20 Hz, broadcast StateSnapshot
    └── client/             # Bevy 0.19 + WebGL2 — compile en WASM
        ├── src/
        │   ├── main.rs     # App Bevy avec DefaultPlugins + plugins custom
        │   ├── network.rs  # WsConnection (NonSend), poll WebSocket, GameState
        │   └── render.rs   # Camera2d, sprite sous-marin, update position/cap
        ├── index.html      # page HTML servie par Trunk
        └── Trunk.toml      # serve :8080
```

### Règles de dépendances

- `shared` ne dépend que de `serde` et `postcard`
- `simulation` dépend uniquement de `shared` — jamais de Bevy ni d'Axum
- `server` dépend de `shared` + `simulation` + Axum + Tokio
- `client` dépend de `shared` + Bevy (jamais de simulation directement)

## Prérequis

| Outil | Version | Installation |
|-------|---------|--------------|
| Rust (stable) | ≥ 1.95 | `rustup update stable` |
| target WASM | — | `rustup target add wasm32-unknown-unknown` |
| Trunk | 0.20+ | `cargo install trunk --locked` |

## Démarrage rapide

```bash
# Terminal 1 — serveur de jeu
make server

# Terminal 2 — client WASM (hot-reload)
make client

# Navigateur
open http://127.0.0.1:8080
```

Le premier build du client prend ~2 minutes (compilation de Bevy pour WASM). Les builds suivants sont quasi-instantanés grâce au cache Cargo.

## Commandes disponibles

```bash
make server        # cargo run -p server (port 3000)
make client        # trunk serve (port 8080)
make check         # cargo check shared + simulation + server (natif)
make check-client  # cargo check client --target wasm32-unknown-unknown
make check-all     # les deux
make test          # cargo test -p simulation
make build-wasm    # trunk build --release → crates/client/dist/
make clean         # supprime target/ + dist/ + .trunk/
```

## Protocole réseau

Tous les messages sont sérialisés en binaire avec `postcard` sur WebSocket.

### Client → Serveur (`ClientMessage`)

```rust
JoinLobby { role: CrewRole }   // première connexion
Command(PlayerCommand)          // en jeu
```

### Serveur → Client (`ServerMessage`)

```rust
JoinAck { player_id: u32, role: CrewRole }  // confirmation de rôle
GameStarted                                   // les 5 rôles sont remplis
Event(GameEvent)                              // snapshot d'état ou événement
Error(ProtocolError)                          // rôle pris, commande rejetée…
```

### Tick rate

20 ticks/s côté serveur. Le client interpolera entre les états reçus pour rendre à 60 fps (à implémenter).

## État du sous-marin (`SubmarineState`)

```rust
pub struct SubmarineState {
    pub x: f32,              // position horizontale (carte)
    pub y: f32,              // position horizontale (carte)
    pub depth: f32,          // profondeur en mètres (0 = surface)
    pub heading: f32,        // cap 0–360°
    pub speed: f32,          // nœuds
    pub hull_integrity: f32, // 0–100
    pub systems: Vec<(SystemId, SystemStatus)>,
}
```

## Systèmes du sous-marin

`Engine` · `Torpedo` · `Sonar` · `Life` · `Navigation`

Chaque système a un état `operational: bool` et un niveau d'énergie `power: f32`.

## Déploiement

Cible : conteneur unique (binaire Rust) derrière un AWS Application Load Balancer avec terminaison HTTPS/WSS, sur ECS Fargate. L'ALB supporte les upgrades WebSocket nativement.

```bash
# Build release server
cargo build -p server --release

# Build release client WASM
make build-wasm
# → artefacts dans crates/client/dist/ à servir comme fichiers statiques
```

## Roadmap

- [ ] Physique du sous-marin (`simulation::tick`)
- [ ] UI de station (sélecteur de rôle, jauges, commandes)
- [ ] Interpolation client entre les StateSnapshot
- [ ] Validation des commandes par rôle (`game_room.rs`)
- [ ] Reconnexion avec snapshot d'état complet
- [ ] Dockerfile + déploiement ECS Fargate
