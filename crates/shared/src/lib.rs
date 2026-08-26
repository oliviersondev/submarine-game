use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrewRole {
    Captain,
    Pilot,
    Sonar,
    Engineer,
    Weapons,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemId {
    Engine,
    Torpedo,
    Sonar,
    Life,
    Navigation,
}

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
pub struct SubmarineState {
    pub heading: f32,
    pub depth: f32,
    pub speed: f32,
    pub hull_integrity: f32,
}

impl Default for SubmarineState {
    fn default() -> Self {
        Self {
            heading: 0.0,
            depth: 0.0,
            speed: 0.0,
            hull_integrity: 100.0,
        }
    }
}
