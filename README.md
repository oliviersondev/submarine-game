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
| `Sonar` | Détection, classification et partage des pistes |
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
| `Weapons` | Contrôles tactiles : piste, tube, réglages, préparation et confirmation du tir |
| `Captain` | Ordre au bot Pilote, ordre de préparation au bot Armement et autorisation du tir |

Le HUD affiche le code de salle, le tick serveur, le bruit propre, les alertes actives et la dernière erreur. Le poste Pilote distingue consignes et mesures réelles et contrôle les ballasts ainsi que la remontée d'urgence. Le poste Ingénierie contrôle propulsion, ventilation et recharge. Le Sonar suit des observations imparfaites, gère ses pistes et les partage avec le Capitaine et l'Armement. Leur carte tactique ne montre que ces estimations partagées. Le Capitaine peut aussi transmettre au bot Pilote une consigne de cap, vitesse et profondeur.

### Essayer une attaque M4

Le **[guide joueur Armement](docs/guide-armement.md)** explique les mesures de l'écran, chaque bouton, la procédure complète et les messages de blocage.

1. Créer une salle au poste `Weapons` (`?role=weapons`), se déclarer prêt et démarrer. Le bot Sonar partage ses pistes lorsqu'elles deviennent fiables.
2. Sélectionner une piste et un tube avant ; **Charger**, attendre 20 s, puis **Configurer**. Le bouton de réglage alterne profondeur de torpille, correction du cap cible et correction de sa vitesse ; `−` et `+` modifient le réglage, puis **Configurer** l'applique.
3. **Inonder** (8 s), puis **Ouvrir** (3 s). Le bot Capitaine autorise une solution disponible. Un Capitaine humain utilise **Autoriser** sur le tube configuré.
4. Le sous-marin doit être à 20 m ou moins et la solution dans le demi-plan du tube (avant, ou arrière pour le tube 5). **Tirer…**, puis **CONFIRMER TIR** lance une torpille non guidée à 40 nœuds, limitée à 8 km.
5. Après le tir, la porte se ferme automatiquement (3 s) ; **Purger** (8 s) rend le tube rechargeable. Il existe cinq torpilles au total, réparties entre réserve et tubes.

Au poste Capitaine, sélectionner une piste puis **Ordre bot** prépare une attaque si l'Armement est vacant ; **Autoriser** déclenche ensuite le tir du bot prêt. Sans humain aux postes Capitaine et Armement, le bot Capitaine désigne une piste marchande fiable. Les opérations de tube produisent du bruit. Les panneaux d'armement se parcourent par glissement si leur contenu dépasse l'écran. Les explosions ne sont rapportées qu'à portée d'écoute ; elles ne révèlent ni identité, ni position exacte, ni intégrité réelle d'un ennemi.

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
    │   └── src/            # navigation, monde privé, détection, pistes et weapons.rs
    ├── server/             # Axum 0.8 + Tokio — binaire natif
    │   └── src/
    │       ├── main.rs     # écoute 0.0.0.0:3000, route /ws
    │       ├── lobby.rs    # registre de salles, rôles, prêts, démarrage, ws_handler
    │       └── game_room.rs# boucle de jeu 20 Hz et projections par rôle
    └── client/             # Bevy 0.19 + WebGL2 — compile en WASM
        ├── src/
        │   ├── main.rs     # App Bevy avec DefaultPlugins + plugins custom
        │   ├── network.rs  # WsConnection (NonSend), poll WebSocket, GameState
        │   ├── render.rs   # Camera2d, stations tactiles et carte
        │   └── weapons_ui.rs # préparation, réglages et confirmation du tir
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

Chaque enveloppe porte `version: u16` (actuellement `5`). `RoomId`, `SessionId`, `PlayerId`, `CommandId` et `TrackId` sont des types opaques. Les commandes sont séparées entre lobby et mission. La version 4 a introduit les commandes explicites d'armement, la configuration des torpilles, l'autorisation du Capitaine, les projections des tubes et les accusés de réception. La version 5 remplace la version 4 et ajoute les causes structurées d'échec des solutions de tir, transmises uniquement aux postes autorisés. Serveur et client doivent être mis à jour ensemble.

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
Snapshot { snapshot_id, tick, mission }
Event { tick, event }
CommandAccepted { command_id }
Error { command_id, error }
```

### Tick rate

20 ticks/s côté serveur et par salle. Chaque mission reçoit une graine et une configuration déterministes, et chaque snapshot contient ses numéros de snapshot et de tick. Le tick avance le sous-marin et le convoi, puis met à jour ressources, bruit, observations et pistes. Le client ignore les anciens snapshots et interpole la position et le cap entre les deux derniers reçus.

## État du sous-marin

`SubmarineState` est l'état autoritaire du bâtiment joueur. Le monde ennemi reste privé dans `simulation`. Le réseau transmet un `MissionSnapshot` contenant la projection du sous-marin et, selon le rôle, les observations Sonar ou les seules pistes explicitement partagées. Aucune position ennemie réelle ne traverse le protocole.

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
- [ ] Interfaces des cinq stations finalisées sur mobile (postes minimaux et procédures M4 livrés)
- [x] Interpolation client entre les snapshots projetés
- [x] Validation des commandes par rôle (`game_room.rs`)
- [x] Salles éphémères, démarrage solo et bots de postes vacants (M1)
- [x] Protocole v5 avec fixtures `postcard`, projections par rôle, diagnostics d'armement, accusés et ticks numérotés
- [x] Navigation inertielle, plongée, endurance, bruit et alertes (M2)
- [x] Convoi, observations, pistes incertaines et poste Sonar tactile (M3)
- [x] Tubes, solution de tir, autorisation, torpilles et impacts simulés (M4)
- [ ] Reconnexion avec snapshot d'état complet
- [ ] Dockerfile + déploiement ECS Fargate
