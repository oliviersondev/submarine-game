use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CrewRole {
    Captain,
    Pilot,
    Sonar,
    Engineer,
    Weapons,
}

impl CrewRole {
    pub const ALL: [Self; 5] = [
        Self::Captain,
        Self::Pilot,
        Self::Sonar,
        Self::Engineer,
        Self::Weapons,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemId {
    Engine,
    Torpedo,
    Sonar,
    Life,
    Navigation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemStatus {
    pub operational: bool,
    pub power: f32,
}

impl Default for SystemStatus {
    fn default() -> Self {
        Self {
            operational: true,
            power: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmarineState {
    pub x: f32,
    pub y: f32,
    pub depth: f32,
    pub heading: f32,
    pub speed: f32,
    pub hull_integrity: f32,
    pub systems: Vec<(SystemId, SystemStatus)>,
}

impl Default for SubmarineState {
    fn default() -> Self {
        use SystemId::*;
        Self {
            x: 0.0,
            y: 0.0,
            depth: 0.0,
            heading: 0.0,
            speed: 0.0,
            hull_integrity: 100.0,
            systems: vec![
                (Engine, SystemStatus::default()),
                (Torpedo, SystemStatus::default()),
                (Sonar, SystemStatus::default()),
                (Life, SystemStatus::default()),
                (Navigation, SystemStatus::default()),
            ],
        }
    }
}
