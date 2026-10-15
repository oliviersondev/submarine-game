use serde::{Deserialize, Serialize};

use crate::state::{BallastState, CrewRole, MissionSnapshot, SubmarineConfig, SystemId, TrackId};

pub const PROTOCOL_VERSION: u16 = 5;

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
}

impl MissionConfig {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            submarine: SubmarineConfig::default(),
            weapons: crate::WeaponsConfig::default(),
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
    TorpedoFired { bearing: f32 },
    TorpedoImpact { torpedo_id: u64 },
    TorpedoExpired { torpedo_id: u64 },
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
    EqualTargetAndTorpedoSpeed,
    InterceptOutOfRange,
    TargetOutsideTubeSector,
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
    fn version_five_fixtures_detect_enum_reordering() {
        let create = ClientMessage::new(ClientPayload::Lobby(LobbyCommand::CreateRoom {
            role: CrewRole::Captain,
        }));
        let ready =
            ClientMessage::new(ClientPayload::Lobby(LobbyCommand::SetReady { ready: true }));

        assert_eq!(encode(&create), vec![5, 0, 0, 0]);
        assert_eq!(encode(&ready), vec![5, 0, 2, 1]);
        assert_eq!(decode::<ClientMessage>(&[5, 0, 0, 0]).unwrap(), create);
        assert_eq!(decode::<ClientMessage>(&[5, 0, 2, 1]).unwrap(), ready);
    }

    #[test]
    fn version_five_m2_command_fixture_is_stable() {
        let message = ClientMessage::new(ClientPayload::Mission(MissionCommand::Player {
            command_id: CommandId(9),
            command: PlayerCommand::SetBallast(BallastState::Flood),
        }));
        let fixture = vec![5, 1, 0, 9, 3, 0];
        assert_eq!(encode(&message), fixture);
        assert_eq!(decode::<ClientMessage>(&fixture).unwrap(), message);
    }

    #[test]
    fn version_five_weapons_and_acknowledgment_fixtures_are_stable() {
        let load = ClientMessage::new(ClientPayload::Mission(MissionCommand::Player {
            command_id: CommandId(9),
            command: PlayerCommand::LoadTube(0),
        }));
        let fire = ClientMessage::new(ClientPayload::Mission(MissionCommand::Player {
            command_id: CommandId(9),
            command: PlayerCommand::FireTorpedo { tube: 0 },
        }));
        for (message, fixture) in [
            (load, vec![5, 1, 0, 9, 15, 0]),
            (fire, vec![5, 1, 0, 9, 9, 0]),
        ] {
            assert_eq!(encode(&message), fixture);
            assert_eq!(decode::<ClientMessage>(&fixture).unwrap(), message);
        }
        let ack = ServerMessage::new(ServerPayload::CommandAccepted {
            command_id: CommandId(9),
        });
        assert_eq!(encode(&ack), vec![5, 0, 9]);
        assert_eq!(decode::<ServerMessage>(&[5, 0, 9]).unwrap(), ack);
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
    fn version_five_mission_snapshot_round_trips() {
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
}
