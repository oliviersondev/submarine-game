use serde::{Deserialize, Serialize};

/// P2 enemy lookout tuning; metres, seconds, degrees and confidence points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PeriscopeDetectionConfig {
    pub escort: LookoutConfig,
    pub cargo: LookoutConfig,
    /// Seconds to forget a full, unfinished acquisition without visibility.
    pub memory_time: f32,
    /// Acquisition multiplier at zero distance (1 at maximum range).
    pub proximity_multiplier: f32,
    /// Additional acquisition multiplier per knot of submarine speed.
    pub speed_factor: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LookoutConfig {
    pub range: f32,
    /// Exposure needed at maximum range with a stationary submarine.
    pub acquisition_time: f32,
    pub bearing_error: f32,
    pub range_error: f32,
    pub confidence: f32,
}

impl Default for PeriscopeDetectionConfig {
    fn default() -> Self {
        Self {
            escort: LookoutConfig {
                range: 3_000.0,
                acquisition_time: 12.0,
                bearing_error: 5.0,
                range_error: 0.25,
                confidence: 60.0,
            },
            cargo: LookoutConfig {
                range: 2_000.0,
                acquisition_time: 24.0,
                bearing_error: 9.0,
                range_error: 0.4,
                confidence: 40.0,
            },
            memory_time: 10.0,
            proximity_multiplier: 3.0,
            speed_factor: 0.15,
        }
    }
}

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
