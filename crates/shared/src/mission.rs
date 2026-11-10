use serde::{Deserialize, Serialize};

use crate::SilhouetteKind;

/// Public mission information, never an enemy identity or position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MissionBriefing {
    pub target: SilhouetteKind,
    pub extraction: ExtractionZone,
    /// Square operation area, in metres from the origin on each axis.
    pub operation_half_extent: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExtractionZone {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}

impl MissionBriefing {
    pub fn level_one(seed: u64) -> Self {
        Self {
            target: if seed % 2 == 0 {
                SilhouetteKind::SingleFunnelCargo
            } else {
                SilhouetteKind::TwinFunnelCargo
            },
            extraction: ExtractionZone {
                x: 0.0,
                y: -2_000.0,
                radius: 500.0,
            },
            operation_half_extent: 7_000.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionDefeat {
    SubmarineLost,
    TargetEscaped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionPhase {
    Attack,
    Extract,
    Victory,
    Defeat(MissionDefeat),
}

impl MissionPhase {
    pub fn finished(self) -> bool {
        matches!(self, Self::Victory | Self::Defeat(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MissionProgress {
    pub briefing: MissionBriefing,
    pub phase: MissionPhase,
    pub elapsed_seconds: f64,
}
