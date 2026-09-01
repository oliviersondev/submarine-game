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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiveState {
    Surface,
    Periscope,
    Submerged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BallastState {
    Flood,
    Hold,
    Blow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AcousticLevel {
    Silent,
    Low,
    Notable,
    Loud,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertKind {
    BatteryLow,
    AirCritical,
    Cavitation,
    CriticalDepth,
}

pub const ALERT_KINDS: [AlertKind; 4] = [
    AlertKind::BatteryLow,
    AlertKind::AirCritical,
    AlertKind::Cavitation,
    AlertKind::CriticalDepth,
];

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SubmarineConfig {
    pub max_surface_speed: f32,
    pub max_submerged_speed: f32,
    pub silent_speed: f32,
    pub periscope_depth: f32,
    pub operational_depth: f32,
    pub critical_depth: f32,
    pub crush_depth: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub turn_rate: f32,
    pub vertical_speed: f32,
    pub emergency_ascent_speed: f32,
    pub battery_drain_base: f32,
    pub battery_drain_at_full_speed: f32,
    pub battery_recharge_rate: f32,
    pub oxygen_drain_rate: f32,
    pub oxygen_ventilation_rate: f32,
    pub low_battery_threshold: f32,
    pub critical_air_threshold: f32,
}

impl Default for SubmarineConfig {
    fn default() -> Self {
        Self {
            max_surface_speed: 18.0,
            max_submerged_speed: 8.0,
            silent_speed: 2.0,
            periscope_depth: 12.0,
            operational_depth: 150.0,
            critical_depth: 220.0,
            crush_depth: 250.0,
            acceleration: 0.75,
            deceleration: 1.25,
            turn_rate: 4.0,
            vertical_speed: 1.5,
            emergency_ascent_speed: 3.0,
            battery_drain_base: 0.01,
            battery_drain_at_full_speed: 0.18,
            battery_recharge_rate: 0.25,
            oxygen_drain_rate: 0.015,
            oxygen_ventilation_rate: 0.5,
            low_battery_threshold: 20.0,
            critical_air_threshold: 15.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PropulsionState {
    pub diesels_on: bool,
    pub electric_motors_on: bool,
    pub ventilation_on: bool,
    pub charging: bool,
}

impl Default for PropulsionState {
    fn default() -> Self {
        Self {
            diesels_on: true,
            electric_motors_on: true,
            ventilation_on: true,
            charging: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmarineState {
    pub x: f32,
    pub y: f32,
    pub depth: f32,
    pub ordered_depth: f32,
    pub heading: f32,
    pub ordered_heading: f32,
    pub speed: f32,
    pub ordered_speed: f32,
    pub turn_rate: f32,
    pub vertical_speed: f32,
    pub dive_state: DiveState,
    pub ballast: BallastState,
    pub emergency_surface: bool,
    pub propulsion: PropulsionState,
    pub battery: f32,
    pub oxygen: f32,
    pub electrical_load: f32,
    pub acoustic_signature: f32,
    pub acoustic_level: AcousticLevel,
    pub cavitating: bool,
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
            ordered_depth: 0.0,
            heading: 0.0,
            ordered_heading: 0.0,
            speed: 0.0,
            ordered_speed: 0.0,
            turn_rate: 0.0,
            vertical_speed: 0.0,
            dive_state: DiveState::Surface,
            ballast: BallastState::Hold,
            emergency_surface: false,
            propulsion: PropulsionState::default(),
            battery: 100.0,
            oxygen: 100.0,
            electrical_load: 0.0,
            acoustic_signature: 0.0,
            acoustic_level: AcousticLevel::Silent,
            cavitating: false,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommonMeasurements {
    pub x: f32,
    pub y: f32,
    pub heading: f32,
    pub speed: f32,
    pub depth: f32,
    pub dive_state: DiveState,
    pub acoustic_level: AcousticLevel,
    pub alerts: Vec<AlertKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PilotMeasurements {
    pub ordered_heading: f32,
    pub ordered_speed: f32,
    pub ordered_depth: f32,
    pub turn_rate: f32,
    pub vertical_speed: f32,
    pub ballast: BallastState,
    pub emergency_surface: bool,
    pub max_speed: f32,
    pub max_depth: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineeringMeasurements {
    pub propulsion: PropulsionState,
    pub battery: f32,
    pub oxygen: f32,
    pub electrical_load: f32,
    pub air_intake_available: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubmarineSnapshot {
    pub common: CommonMeasurements,
    pub pilot: Option<PilotMeasurements>,
    pub engineering: Option<EngineeringMeasurements>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObservationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObservationMode {
    Passive,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AcousticProfile {
    LowFrequency,
    HighFrequency,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContactClassification {
    Unknown,
    Merchant,
    Escort,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SonarObservation {
    pub id: ObservationId,
    pub tick: u64,
    pub mode: ObservationMode,
    pub bearing: f32,
    pub bearing_uncertainty: f32,
    pub distance: Option<f32>,
    pub distance_uncertainty: Option<f32>,
    pub signal_strength: f32,
    pub profile: AcousticProfile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackEstimate {
    pub id: TrackId,
    pub bearing: f32,
    pub bearing_uncertainty: f32,
    pub distance: Option<f32>,
    pub distance_uncertainty: Option<f32>,
    pub heading: Option<f32>,
    pub speed: Option<f32>,
    pub classification: ContactClassification,
    pub confidence: f32,
    pub last_observation_tick: u64,
    pub shared: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SonarMeasurements {
    pub observations: Vec<SonarObservation>,
    pub tracks: Vec<TrackEstimate>,
    pub own_noise: f32,
    pub ping_cooldown_remaining: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TacticalMeasurements {
    pub shared_tracks: Vec<TrackEstimate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionSnapshot {
    pub submarine: SubmarineSnapshot,
    pub sonar: Option<SonarMeasurements>,
    pub tactical: Option<TacticalMeasurements>,
}
