use serde::{Deserialize, Serialize};

use crate::state::{CrewRole, SubmarineState, SystemId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerCommand {
    SetHeading(f32),
    SetDepth(f32),
    SetSpeed(f32),
    FireTorpedo { bearing: f32 },
    RepairSystem(SystemId),
    SonarPing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    StateSnapshot(SubmarineState),
    SonarContact { bearing: f32, distance: f32 },
    SystemDamaged(SystemId),
    SystemRepaired(SystemId),
    TorpedoFired { bearing: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolError {
    RoleAlreadyTaken(CrewRole),
    CommandNotAllowedForRole,
    GameNotStarted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    JoinLobby { role: CrewRole },
    Command(PlayerCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    JoinAck { player_id: u32, role: CrewRole },
    GameStarted,
    Event(GameEvent),
    Error(ProtocolError),
}
