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

impl SystemId {
    pub const ALL: [Self; 5] = [
        Self::Engine,
        Self::Torpedo,
        Self::Sonar,
        Self::Life,
        Self::Navigation,
    ];

    pub const fn compartment(self) -> u8 {
        match self {
            Self::Torpedo => 0,
            Self::Sonar | Self::Navigation => 1,
            Self::Engine | Self::Life => 2,
        }
    }
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
    HullDamaged,
    Flooding,
    SubmarineDestroyed,
    Fire,
    SystemFailure,
}

pub const ALERT_KINDS: [AlertKind; 9] = [
    AlertKind::BatteryLow,
    AlertKind::AirCritical,
    AlertKind::Cavitation,
    AlertKind::CriticalDepth,
    AlertKind::HullDamaged,
    AlertKind::Flooding,
    AlertKind::SubmarineDestroyed,
    AlertKind::Fire,
    AlertKind::SystemFailure,
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
    pub damage: DamageSummary,
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
    pub compartments: [CompartmentState; 3],
    /// Bounded authoritative history, independent of transient event delivery.
    pub damage_history: Vec<DamageRecord>,
    pub damage_control: DamageControlStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageControlAction {
    SealBreach,
    Extinguish,
    RepairSystem(SystemId),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Intervention {
    pub compartment: u8,
    pub action: DamageControlAction,
    pub remaining: f32,
    pub duration: f32,
    pub paused: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DamageControlStatus {
    pub intervention: Option<Intervention>,
    pub systems: Vec<(SystemId, SystemStatus)>,
    pub faults: Vec<SystemId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageCause {
    DepthCharge,
    Pressure,
    Flooding,
    Fire,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DamageSummary {
    pub hull_integrity: f32,
    pub flooding: f32,
    pub destroyed: bool,
    pub last_cause: Option<DamageCause>,
    pub fire: f32,
    pub unavailable_systems: u8,
}

impl Default for DamageSummary {
    fn default() -> Self {
        Self {
            hull_integrity: 100.0,
            flooding: 0.0,
            destroyed: false,
            last_cause: None,
            fire: 0.0,
            unavailable_systems: 0,
        }
    }
}

/// Fixed order: bow, control room, machinery. All percentages are 0..=100.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CompartmentState {
    pub integrity: f32,
    pub flooding: f32,
    pub breach_rate: f32,
    pub fire: f32,
    pub isolated: bool,
    pub pumping: bool,
}

impl Default for CompartmentState {
    fn default() -> Self {
        Self {
            integrity: 100.0,
            flooding: 0.0,
            breach_rate: 0.0,
            fire: 0.0,
            isolated: false,
            pumping: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DamageRecord {
    pub tick: u64,
    pub cause: DamageCause,
    pub hull_damage: f32,
    pub compartment: Option<u8>,
    pub destroyed: bool,
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
    pub operational: bool,
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
    pub weapons: Option<WeaponsMeasurements>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TubePhase {
    Empty,
    Loading,
    Loaded,
    Flooding,
    Flooded,
    Opening,
    Ready,
    Closing,
    Purging,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShotSettings {
    pub target: TrackId,
    pub depth: f32,
    pub heading_correction: f32,
    pub speed_correction: f32,
}

impl ShotSettings {
    pub fn for_target(target: TrackId) -> Self {
        Self {
            target,
            depth: 4.0,
            heading_correction: 0.0,
            speed_correction: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TubeMeasurements {
    pub phase: TubePhase,
    pub loaded: bool,
    pub remaining_seconds: f32,
    pub settings: Option<ShotSettings>,
    pub authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FiringSolution {
    pub bearing: f32,
    pub run_seconds: f32,
    pub uncertainty_meters: f32,
    pub warnings: Vec<String>,
    pub launch_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TorpedoMeasurements {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub depth: f32,
    pub bearing: f32,
    pub remaining_meters: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeaponsMeasurements {
    pub tubes: Vec<TubeMeasurements>,
    pub reserve: u8,
    pub solutions: Vec<Option<FiringSolution>>,
    pub solution_errors: Vec<Option<crate::ProtocolError>>,
    pub torpedoes: Vec<TorpedoMeasurements>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WeaponsConfig {
    pub load_seconds: f32,
    pub flood_seconds: f32,
    pub door_seconds: f32,
    pub purge_seconds: f32,
    pub torpedo_speed_knots: f32,
    pub torpedo_range_meters: f32,
    pub damage: f32,
    pub collision_radius_meters: f32,
    pub max_launch_depth: f32,
    pub max_surface_impact_depth: f32,
}

impl Default for WeaponsConfig {
    fn default() -> Self {
        Self {
            load_seconds: 20.0,
            flood_seconds: 8.0,
            door_seconds: 3.0,
            purge_seconds: 8.0,
            torpedo_speed_knots: 40.0,
            torpedo_range_meters: 8000.0,
            damage: 100.0,
            collision_radius_meters: 35.0,
            max_launch_depth: 20.0,
            max_surface_impact_depth: 8.0,
        }
    }
}
