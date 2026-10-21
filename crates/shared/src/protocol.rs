use serde::{Deserialize, Serialize};

use crate::state::{BallastState, CrewRole, MissionSnapshot, SubmarineConfig, SystemId, TrackId};

pub const PROTOCOL_VERSION: u16 = 8;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MissionConfig {
    pub seed: u64,
    pub submarine: SubmarineConfig,
    pub weapons: crate::WeaponsConfig,
    pub escort: EscortConfig,
    pub damage: DamageConfig,
}

impl MissionConfig {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            submarine: SubmarineConfig::default(),
            weapons: crate::WeaponsConfig::default(),
            escort: EscortConfig::default(),
            damage: DamageConfig::default(),
        }
    }
}

/// Provisional M5 tuning. Distances in metres, times in seconds, speeds in knots.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EscortConfig {
    pub passive_range: f32,
    pub ping_hearing_range: f32,
    pub active_range: f32,
    pub explosion_hearing_range: f32,
    pub passive_interval: f32,
    pub ping_interval: f32,
    pub track_lifetime: f32,
    pub suspicion_duration: f32,
    pub search_duration: f32,
    pub return_grace: f32,
    /// Confidence points lost per second (confidence is bounded to 0..=100).
    pub confidence_decay: f32,
    pub search_confidence: f32,
    /// Metres of additional uncertainty per second without a new observation.
    pub uncertainty_growth: f32,
    pub search_speed: f32,
    pub return_speed: f32,
    /// Degrees per second.
    pub turn_rate: f32,
    /// Knots per second, also used for deceleration.
    pub acceleration: f32,
    pub attack: EscortAttackConfig,
}

impl Default for EscortConfig {
    fn default() -> Self {
        Self {
            passive_range: 6_000.0,
            ping_hearing_range: 10_000.0,
            active_range: 3_000.0,
            explosion_hearing_range: 3_000.0,
            passive_interval: 1.0,
            ping_interval: 12.0,
            track_lifetime: 60.0,
            suspicion_duration: 30.0,
            search_duration: 180.0,
            return_grace: 20.0,
            confidence_decay: 1.0,
            search_confidence: 55.0,
            uncertainty_growth: 8.0,
            search_speed: 8.0,
            return_speed: 15.0,
            turn_rate: 5.0,
            acceleration: 0.75,
            attack: EscortAttackConfig::default(),
        }
    }
}

/// Enemy doctrine: preset depths, never the player's actual depth.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EscortAttackConfig {
    pub confidence: f32,
    pub max_uncertainty: f32,
    pub max_track_age: f32,
    pub release_range: f32,
    pub cooldown: f32,
    pub salvos: u8,
    pub depth: f32,
    pub depth_spacing: f32,
    pub sink_speed: f32,
    pub arming_time: f32,
    pub blast_radius: f32,
    pub hearing_range: f32,
    pub damage: f32,
}

impl Default for EscortAttackConfig {
    fn default() -> Self {
        Self {
            confidence: 70.0,
            max_uncertainty: 350.0,
            max_track_age: 15.0,
            release_range: 120.0,
            cooldown: 30.0,
            salvos: 6,
            depth: 100.0,
            depth_spacing: 40.0,
            sink_speed: 5.0,
            arming_time: 3.0,
            blast_radius: 120.0,
            hearing_range: 3_000.0,
            damage: 24.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DamageConfig {
    /// Compartment flooding percentage/second added per hull damage point.
    pub breach_per_damage: f32,
    /// Downward drift in m/s at 100% mean flooding.
    pub flooding_sink_speed: f32,
    pub crush_damage_per_second: f32,
}

impl Default for DamageConfig {
    fn default() -> Self {
        Self {
            breach_per_damage: 0.003,
            flooding_sink_speed: 4.0,
            crush_damage_per_second: 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PilotOrder {
    pub heading: f32,
    pub speed: f32,
    pub depth: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlayerCommand {
    SetHeading(f32),
    SetDepth(f32),
    SetSpeed(f32),
    SetBallast(BallastState),
    EmergencySurface,
    SetDiesels(bool),
    SetElectricMotors(bool),
    SetVentilation(bool),
    SetBatteryCharging(bool),
    FireTorpedo {
        tube: u8,
    },
    RepairSystem(SystemId),
    SonarPing,
    MergeTracks {
        primary: TrackId,
        secondary: TrackId,
    },
    SetTrackShared {
        track_id: TrackId,
        shared: bool,
    },
    DropTrack(TrackId),
    LoadTube(u8),
    ConfigureShot {
        tube: u8,
        settings: crate::ShotSettings,
    },
    FloodTube(u8),
    OpenTube(u8),
    CloseTube(u8),
    PurgeTube(u8),
    AuthorizeShot {
        tube: u8,
        settings: crate::ShotSettings,
    },
    OrderWeaponsBot {
        target: TrackId,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameEvent {
    AlertRaised(crate::AlertKind),
    AlertCleared(crate::AlertKind),
    EmergencySurfaceStarted,
    SystemDamaged(SystemId),
    SystemRepaired(SystemId),
    TorpedoFired {
        bearing: f32,
    },
    TorpedoImpact {
        torpedo_id: u64,
    },
    TorpedoExpired {
        torpedo_id: u64,
    },
    /// An audible emission, not confirmation of an enemy echo or identification.
    SonarPingHeard {
        bearing: f32,
        bearing_uncertainty: f32,
    },
    /// Audible blast only: no weapon/source identity, coordinates or hit result.
    /// Wire index 9 is unchanged from the former `DepthChargeHeard` name.
    ExplosionHeard,
    SubmarineDamaged {
        cause: crate::DamageCause,
    },
    SubmarineDestroyed {
        cause: crate::DamageCause,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProtocolError {
    IncompatibleVersion { expected: u16, received: u16 },
    RoomNotFound,
    RoomAlreadyStarted,
    RoleAlreadyTaken(CrewRole),
    CommandNotAllowedForRole,
    GameNotStarted,
    PilotControlledByHuman,
    InvalidRoomCode,
    ConnectionFailed,
    TrackNotFound(TrackId),
    InvalidTrackMerge,
    SonarPingCoolingDown,
    InvalidTube,
    InvalidTubePhase,
    NoTorpedoes,
    InvalidShotSettings,
    TrackNotShared,
    NoFiringSolution,
    ShotNotAuthorized,
    LaunchDepthExceeded,
    WeaponsControlledByHuman,
    MissingTargetDistance,
    MissingTargetHeading,
    MissingTargetSpeed,
    InterceptOutOfRange,
    TargetOutsideTubeSector,
    AuthorizationSettingsMismatch,
    TorpedoDepthExceedsImpactZone { maximum_meters: f32 },
    SubmarineDestroyed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientMessage {
    pub version: u16,
    pub payload: ClientPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientPayload {
    Lobby(LobbyCommand),
    Mission(MissionCommand),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LobbyCommand {
    CreateRoom { role: CrewRole },
    JoinRoom { room_id: RoomId, role: CrewRole },
    SetReady { ready: bool },
    StartMission,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MissionCommand {
    Player {
        command_id: CommandId,
        command: PlayerCommand,
    },
    OrderPilotBot {
        command_id: CommandId,
        order: PilotOrder,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerMessage {
    pub version: u16,
    pub payload: ServerPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerPayload {
    CommandAccepted {
        command_id: CommandId,
    },
    SessionJoined {
        session_id: SessionId,
        player_id: PlayerId,
        room_id: RoomId,
        role: CrewRole,
    },
    Lobby(LobbySnapshot),
    MissionStarted {
        config: MissionConfig,
    },
    Snapshot {
        snapshot_id: u64,
        tick: u64,
        mission: MissionSnapshot,
    },
    Event {
        tick: u64,
        event: GameEvent,
    },
    Error {
        command_id: Option<CommandId>,
        error: ProtocolError,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LobbyPhase {
    Waiting,
    Running,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleOccupant {
    Human { player_id: PlayerId, ready: bool },
    Bot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleSlot {
    pub role: CrewRole,
    pub occupant: RoleOccupant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbySnapshot {
    pub room_id: RoomId,
    pub phase: LobbyPhase,
    pub slots: Vec<RoleSlot>,
}

impl ClientMessage {
    pub fn new(payload: ClientPayload) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            payload,
        }
    }
}

impl ServerMessage {
    pub fn new(payload: ServerPayload) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::{decode, encode};

    #[test]
    fn protocol_messages_round_trip() {
        let messages = [
            ClientMessage::new(ClientPayload::Lobby(LobbyCommand::CreateRoom {
                role: CrewRole::Captain,
            })),
            ClientMessage::new(ClientPayload::Lobby(LobbyCommand::JoinRoom {
                room_id: RoomId("A1B2C3".into()),
                role: CrewRole::Pilot,
            })),
            ClientMessage::new(ClientPayload::Mission(MissionCommand::OrderPilotBot {
                command_id: CommandId(42),
                order: PilotOrder {
                    heading: 90.0,
                    speed: 8.0,
                    depth: 40.0,
                },
            })),
            ClientMessage::new(ClientPayload::Mission(MissionCommand::Player {
                command_id: CommandId(43),
                command: PlayerCommand::SetBallast(BallastState::Flood),
            })),
        ];

        for message in messages {
            assert_eq!(decode::<ClientMessage>(&encode(&message)).unwrap(), message);
        }
    }

    #[test]
    fn version_eight_fixtures_detect_enum_reordering() {
        let create = ClientMessage::new(ClientPayload::Lobby(LobbyCommand::CreateRoom {
            role: CrewRole::Captain,
        }));
        let ready =
            ClientMessage::new(ClientPayload::Lobby(LobbyCommand::SetReady { ready: true }));

        assert_eq!(encode(&create), vec![8, 0, 0, 0]);
        assert_eq!(encode(&ready), vec![8, 0, 2, 1]);
        assert_eq!(decode::<ClientMessage>(&[8, 0, 0, 0]).unwrap(), create);
        assert_eq!(decode::<ClientMessage>(&[8, 0, 2, 1]).unwrap(), ready);
    }

    #[test]
    fn version_eight_m2_command_fixture_is_stable() {
        let message = ClientMessage::new(ClientPayload::Mission(MissionCommand::Player {
            command_id: CommandId(9),
            command: PlayerCommand::SetBallast(BallastState::Flood),
        }));
        let fixture = vec![8, 1, 0, 9, 3, 0];
        assert_eq!(encode(&message), fixture);
        assert_eq!(decode::<ClientMessage>(&fixture).unwrap(), message);
    }

    #[test]
    fn version_eight_weapons_and_acknowledgment_fixtures_are_stable() {
        let load = ClientMessage::new(ClientPayload::Mission(MissionCommand::Player {
            command_id: CommandId(9),
            command: PlayerCommand::LoadTube(0),
        }));
        let fire = ClientMessage::new(ClientPayload::Mission(MissionCommand::Player {
            command_id: CommandId(9),
            command: PlayerCommand::FireTorpedo { tube: 0 },
        }));
        for (message, fixture) in [
            (load, vec![8, 1, 0, 9, 15, 0]),
            (fire, vec![8, 1, 0, 9, 9, 0]),
        ] {
            assert_eq!(encode(&message), fixture);
            assert_eq!(decode::<ClientMessage>(&fixture).unwrap(), message);
        }
        let ack = ServerMessage::new(ServerPayload::CommandAccepted {
            command_id: CommandId(9),
        });
        assert_eq!(encode(&ack), vec![8, 0, 9]);
        assert_eq!(decode::<ServerMessage>(&[8, 0, 9]).unwrap(), ack);
        let configured = ClientMessage::new(ClientPayload::Mission(MissionCommand::Player {
            command_id: CommandId(10),
            command: PlayerCommand::ConfigureShot {
                tube: 4,
                settings: crate::ShotSettings::for_target(TrackId(42)),
            },
        }));
        assert_eq!(
            decode::<ClientMessage>(&encode(&configured)).unwrap(),
            configured
        );
    }

    #[test]
    fn version_eight_mission_snapshot_round_trips() {
        let message = ServerMessage::new(ServerPayload::Snapshot {
            snapshot_id: 3,
            tick: 7,
            mission: MissionSnapshot {
                submarine: crate::SubmarineSnapshot {
                    common: crate::CommonMeasurements {
                        x: 1.0,
                        y: 2.0,
                        heading: 90.0,
                        speed: 3.0,
                        depth: 12.0,
                        dive_state: crate::DiveState::Periscope,
                        acoustic_level: crate::AcousticLevel::Low,
                        alerts: vec![crate::AlertKind::BatteryLow],
                        damage: crate::DamageSummary::default(),
                    },
                    pilot: Some(crate::PilotMeasurements {
                        ordered_heading: 90.0,
                        ordered_speed: 4.0,
                        ordered_depth: 20.0,
                        turn_rate: 1.0,
                        vertical_speed: 0.5,
                        ballast: BallastState::Flood,
                        emergency_surface: false,
                        max_speed: 8.0,
                        max_depth: 250.0,
                    }),
                    engineering: None,
                },
                sonar: None,
                tactical: None,
                weapons: None,
            },
        });
        assert_eq!(decode::<ServerMessage>(&encode(&message)).unwrap(), message);
    }

    #[test]
    fn version_eight_ping_fixture_and_mission_config_round_trip() {
        let ping = ServerMessage::new(ServerPayload::Event {
            tick: 42,
            event: GameEvent::SonarPingHeard {
                bearing: 90.0,
                bearing_uncertainty: 3.0,
            },
        });
        let fixture = vec![8, 5, 42, 8, 0, 0, 180, 66, 0, 0, 64, 64];
        assert_eq!(encode(&ping), fixture);
        assert_eq!(decode::<ServerMessage>(&fixture).unwrap(), ping);
        let message = ServerMessage::new(ServerPayload::MissionStarted {
            config: MissionConfig::new(123),
        });
        assert_eq!(decode::<ServerMessage>(&encode(&message)).unwrap(), message);
    }

    #[test]
    fn version_eight_damage_events_have_stable_fixtures() {
        for (event, fixture) in [
            (GameEvent::ExplosionHeard, vec![8, 5, 42, 9]),
            (
                GameEvent::SubmarineDamaged {
                    cause: crate::DamageCause::DepthCharge,
                },
                vec![8, 5, 42, 10, 0],
            ),
            (
                GameEvent::SubmarineDestroyed {
                    cause: crate::DamageCause::Flooding,
                },
                vec![8, 5, 42, 11, 2],
            ),
        ] {
            let message = ServerMessage::new(ServerPayload::Event { tick: 42, event });
            assert_eq!(encode(&message), fixture);
            assert_eq!(decode::<ServerMessage>(&fixture).unwrap(), message);
        }
        let engineering = crate::EngineeringMeasurements {
            propulsion: crate::PropulsionState::default(),
            battery: 80.0,
            oxygen: 70.0,
            electrical_load: 0.1,
            air_intake_available: false,
            compartments: [crate::CompartmentState {
                integrity: 80.0,
                flooding: 12.0,
                breach_rate: 0.06,
            }; 3],
            damage_history: vec![crate::DamageRecord {
                tick: 42,
                cause: crate::DamageCause::DepthCharge,
                hull_damage: 20.0,
                compartment: Some(1),
                destroyed: false,
            }],
        };
        assert_eq!(
            decode::<crate::EngineeringMeasurements>(&encode(&engineering)).unwrap(),
            engineering
        );
    }
}
