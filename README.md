# Submarine Game

Jeu coopératif multijoueur 2D de sous-marin en Rust. Les joueurs incarnent différents membres d'équipage et se connectent via WebSocket à une simulation autoritaire côté serveur. Le client compile en WebAssembly et est servi par Trunk ; aucun manifeste ni service worker PWA n'est encore présent.

## Documentation de conception

La direction produit et technique est décrite dans [`docs/`](docs/README.md) :

- [Game Design Document](docs/game-design.md) : vision, rôles et règles de simulation ;
- [Architecture](docs/architecture.md) : état actuel, cible et contraintes techniques ;
- [Roadmap](docs/roadmap.md) : vertical slices et critères de validation.

Les documents distinguent explicitement le prototype actuel des fonctionnalités cibles.

## Vue d'ensemble

Un à cinq joueurs créent ou rejoignent une salle éphémère par son code court, chacun avec un rôle distinct :

| Rôle | Responsabilité |
|------|----------------|
| `Captain` | Coordination générale, décisions tactiques |
| `Pilot` | Cap, vitesse, profondeur |
| `Sonar` | Détection des contacts (contacts bruts non classifiés) |
| `Engineer` | Réparation des systèmes, gestion de l'énergie |
| `Weapons` | Tir des torpilles |

Chaque joueur peut se déclarer prêt. Un joueur démarre explicitement la mission ; les rôles libres sont alors affichés et conservés comme bots. Plusieurs salles indépendantes peuvent fonctionner dans le même processus. Il n'y a ni persistance ni reconnexion à ce stade.

### Contrôles

Les commandes clavier permettent de tester le flux réseau avant l'ajout des interfaces de station :

| Rôle | Touches |
|------|---------|
| `Pilot` | `←`/`→` cap, `↑`/`↓` vitesse, `PageUp`/`PageDown` profondeur |
| `Sonar` | `Espace` ping sonar |
| `Engineer` | `1` diesels, `2` moteurs électriques, `3` ventilation, `4` recharge, `5` réparation moteur |
| `Weapons` | `Espace` tir dans le cap actuel |
| `Captain` | Bouton tactile envoyant un ordre structuré au bot Pilote |

Le HUD affiche le code de salle, le tick serveur, le bruit propre, les alertes actives et la dernière erreur. Le poste Pilote distingue consignes et mesures réelles et contrôle les ballasts ainsi que la remontée d'urgence. Le poste Ingénierie contrôle propulsion, ventilation et recharge tout en suivant batterie, oxygène et charge. Le Capitaine peut transmettre au bot Pilote une consigne de cap, vitesse et profondeur ; cet ordre est explicitement refusé si le Pilote est humain.

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
    │       ├── protocol.rs # enveloppes versionnées, lobby, mission, identifiants et erreurs
    │       └── codec.rs    # encode() / decode() postcard
    ├── simulation/         # logique déterministe — NO async, NO Bevy
    │   └── src/lib.rs      # Simulation::new(), tick(dt), apply_command()
    ├── server/             # Axum 0.8 + Tokio — binaire natif
    │   └── src/
    │       ├── main.rs     # écoute 0.0.0.0:3000, route /ws
    │       ├── lobby.rs    # registre de salles, rôles, prêts, démarrage, ws_handler
    │       └── game_room.rs# boucle de jeu 20 Hz et projections par rôle
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

Sans paramètre d'URL, le client demande un rôle puis permet de créer une salle ou de saisir au clavier ou au pavé tactile les six chiffres d'un code avant d'ouvrir la connexion. Pour les tests rapides, `?role=pilot` (ou un autre nom de `CrewRole`) sélectionne le poste et crée directement une salle.

Depuis Trunk sur le port `8080`, l'URL WebSocket utilise le nom d'hôte de la page et le port de développement `3000`. Sur une autre origine, elle utilise le même hôte, port et un schéma `ws`/`wss` correspondant à la page. En exécution native de développement, `WS_URL` permet de la remplacer.

## Commandes disponibles

```bash
make server        # cargo run -p server (port 3000)
make client        # trunk serve (port 8080)
make check         # cargo check shared + simulation + server (natif)
make check-client  # cargo check client --target wasm32-unknown-unknown
make check-all     # les deux
make test          # cargo test -p shared -p simulation -p server
make build-wasm    # trunk build --release → crates/client/dist/
make clean         # supprime target/ + dist/ + .trunk/
```

## Protocole réseau

Tous les messages sont sérialisés en binaire avec `postcard` sur WebSocket.

Chaque enveloppe porte `version: u16` (actuellement `2`). `RoomId`, `SessionId`, `PlayerId` et `CommandId` sont des types opaques. Les commandes sont séparées entre lobby et mission. La version 2 remplace directement la version 1 et transmet une projection du sous-marin adaptée au rôle plutôt que l'état autoritaire complet.

### Client → Serveur (`ClientPayload`)

```rust
Lobby(CreateRoom | JoinRoom | SetReady | StartMission)
Mission(Player { command_id, command } | OrderPilotBot { command_id, order })
```

### Serveur → Client (`ServerPayload`)

```rust
SessionJoined { session_id, player_id, room_id, role }
Lobby(LobbySnapshot)                          // humains, bots et prêts
MissionStarted { config }
Snapshot { snapshot_id, tick, submarine }
Event { tick, event }
Error { command_id, error }
```

### Tick rate

20 ticks/s côté serveur et par salle. Chaque mission reçoit une graine et une configuration déterministes, et chaque snapshot contient ses numéros de snapshot et de tick. Le tick fait converger les mesures réelles vers les consignes selon des taux bornés, avance le sous-marin avec un cap nautique (`0°` nord, `90°` est), puis met à jour ressources, bruit et alertes. Le client ignore les anciens snapshots et interpole la position et le cap entre les deux derniers reçus.

## État du sous-marin

`SubmarineState` est l'état autoritaire utilisé par `simulation` : position horizontale, profondeur séparée, consignes et mesures réelles, ballasts, propulsion, ressources, bruit, coque et systèmes. Le réseau transmet un `SubmarineSnapshot` contenant les mesures communes et uniquement les détails du poste concerné (`PilotMeasurements` ou `EngineeringMeasurements`).

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

- [x] Déplacement de base du sous-marin (`simulation::tick`)
- [ ] Interfaces des cinq stations complètes (Pilote et Ingénierie M2 livrées)
- [x] Interpolation client entre les snapshots projetés
- [x] Validation des commandes par rôle (`game_room.rs`)
- [x] Salles éphémères, démarrage solo et bots de postes vacants (M1)
- [x] Protocole v2 avec fixtures `postcard`, projections par rôle et ticks numérotés
- [x] Navigation inertielle, plongée, endurance, bruit et alertes (M2)
- [ ] Reconnexion avec snapshot d'état complet
- [ ] Dockerfile + déploiement ECS Fargate
