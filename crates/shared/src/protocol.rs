use serde::{Deserialize, Serialize};

use crate::state::{CrewRole, SubmarineState, SystemId};

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoomId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionConfig {
    pub seed: u64,
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
    FireTorpedo { bearing: f32 },
    RepairSystem(SystemId),
    SonarPing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameEvent {
    SonarContact { bearing: f32, distance: f32 },
    SystemDamaged(SystemId),
    SystemRepaired(SystemId),
    TorpedoFired { bearing: f32 },
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
        submarine: SubmarineState,
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
        ];

        for message in messages {
            assert_eq!(decode::<ClientMessage>(&encode(&message)).unwrap(), message);
        }
    }

    #[test]
    fn version_one_fixtures_detect_enum_reordering() {
        let create = ClientMessage::new(ClientPayload::Lobby(LobbyCommand::CreateRoom {
            role: CrewRole::Captain,
        }));
        let ready =
            ClientMessage::new(ClientPayload::Lobby(LobbyCommand::SetReady { ready: true }));

        assert_eq!(encode(&create), vec![1, 0, 0, 0]);
        assert_eq!(encode(&ready), vec![1, 0, 2, 1]);
        assert_eq!(decode::<ClientMessage>(&[1, 0, 0, 0]).unwrap(), create);
        assert_eq!(decode::<ClientMessage>(&[1, 0, 2, 1]).unwrap(), ready);
    }
}
