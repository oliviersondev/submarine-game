use serde::{Deserialize, Serialize};

/// Initial P1 tuning: metres, knots, degrees and simulation seconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PeriscopeConfig {
    /// Nominal submerged observation depth, in metres.
    pub depth: f32,
    /// Additional immersion allowed: usable from 0 to depth + depth_tolerance.
    pub depth_tolerance: f32,
    pub max_speed: f32,
    pub field_of_view: f32,
    pub range: f32,
    pub old_after: f32,
    pub expires_after: f32,
}

impl Default for PeriscopeConfig {
    fn default() -> Self {
        Self {
            depth: 12.0,
            depth_tolerance: 2.0,
            max_speed: 6.0,
            field_of_view: 40.0,
            range: 8_000.0,
            old_after: 30.0,
            expires_after: 120.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeriscopeBlock {
    Depth,
    Speed,
    Destroyed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SilhouetteKind {
    SingleFunnelCargo,
    TwinFunnelCargo,
    Escort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VisualContactId(pub u64);

/// Only observable drawing measurements, never world coordinates or vessel IDs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualSilhouette {
    /// Ephemeral token, replaced after loss of visibility.
    pub observation_id: u64,
    /// Own contact attached during uninterrupted visibility, never reassociated after loss.
    pub contact_id: Option<VisualContactId>,
    pub bearing: f32,
    pub angular_width: f32,
    pub angular_height: f32,
    /// Signed apparent broadside factor (-1..1), not the vessel's true heading.
    pub aspect: f32,
    pub kind: SilhouetteKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualContact {
    pub id: VisualContactId,
    pub bearing: f32,
    pub kind: SilhouetteKind,
    pub observed_at: f64,
    pub age: f32,
    pub old: bool,
    pub shared: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeriscopeMeasurements {
    pub raised: bool,
    pub bearing: f32,
    pub field_of_view: f32,
    pub blocked: Option<PeriscopeBlock>,
    pub last_auto_lowering: Option<PeriscopeBlock>,
    pub silhouettes: Vec<VisualSilhouette>,
}
