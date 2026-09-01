use shared::{AcousticProfile, ObservationId, ObservationMode, SonarObservation, SubmarineState};

use crate::world::{signed_unit, World};

const PASSIVE_RANGE: f32 = 8_000.0;
const ACTIVE_RANGE: f32 = 12_000.0;

#[derive(Clone, Debug)]
pub(crate) struct DetectionSample {
    pub(crate) observation: SonarObservation,
    pub(crate) range_hint: f32,
    pub(crate) range_uncertainty: f32,
}

pub(crate) fn passive_observations(
    world: &World,
    submarine: &SubmarineState,
    seed: u64,
    tick: u64,
    next_id: &mut u64,
) -> Vec<DetectionSample> {
    observe(
        world,
        submarine,
        seed,
        tick,
        next_id,
        ObservationMode::Passive,
    )
}

pub(crate) fn active_observations(
    world: &World,
    submarine: &SubmarineState,
    seed: u64,
    tick: u64,
    next_id: &mut u64,
) -> Vec<DetectionSample> {
    observe(
        world,
        submarine,
        seed,
        tick,
        next_id,
        ObservationMode::Active,
    )
}

fn observe(
    world: &World,
    submarine: &SubmarineState,
    seed: u64,
    tick: u64,
    next_id: &mut u64,
    mode: ObservationMode,
) -> Vec<DetectionSample> {
    let mut observations = Vec::new();
    for (index, vessel) in world.vessels.iter().enumerate() {
        let dx = vessel.x - submarine.x;
        let dy = vessel.y - submarine.y;
        let distance = dx.hypot(dy);
        let signal = (vessel.signature - distance / 120.0 - submarine.acoustic_signature * 0.8)
            .clamp(0.0, 100.0);
        let in_range = match mode {
            ObservationMode::Passive => distance <= PASSIVE_RANGE && signal >= 12.0,
            ObservationMode::Active => distance <= ACTIVE_RANGE,
        };
        if !in_range {
            continue;
        }

        let channel = tick
            .wrapping_mul(16)
            .wrapping_add(index as u64 * 3)
            .wrapping_add(mode as u64);
        let bearing_uncertainty = match mode {
            ObservationMode::Passive => 2.5 + (100.0 - signal) * 0.06,
            ObservationMode::Active => 0.8,
        };
        let true_bearing = dx.atan2(dy).to_degrees().rem_euclid(360.0);
        let bearing =
            (true_bearing + signed_unit(seed, channel) * bearing_uncertainty).rem_euclid(360.0);
        let (range_hint, internal_range_uncertainty, measured_distance, distance_uncertainty) =
            match mode {
                ObservationMode::Passive => {
                    let uncertainty = (350.0 + distance * 0.06).min(900.0);
                    (
                        (distance + signed_unit(seed, channel + 1) * uncertainty).max(0.0),
                        uncertainty,
                        None,
                        None,
                    )
                }
                ObservationMode::Active => {
                    let uncertainty = 120.0 + distance * 0.015;
                    let measured =
                        (distance + signed_unit(seed, channel + 1) * uncertainty).max(0.0);
                    (measured, uncertainty, Some(measured), Some(uncertainty))
                }
            };
        let id = ObservationId(*next_id);
        *next_id = next_id.wrapping_add(1);
        observations.push(DetectionSample {
            observation: SonarObservation {
                id,
                tick,
                mode,
                bearing,
                bearing_uncertainty,
                distance: measured_distance,
                distance_uncertainty,
                signal_strength: signal,
                profile: observed_profile(vessel.profile(), signal, mode, seed, channel + 2),
            },
            range_hint,
            range_uncertainty: internal_range_uncertainty,
        });
    }
    observations
}

fn observed_profile(
    actual: AcousticProfile,
    signal: f32,
    mode: ObservationMode,
    seed: u64,
    channel: u64,
) -> AcousticProfile {
    let reliability = match mode {
        ObservationMode::Passive => (0.25 + signal * 0.005).clamp(0.25, 0.7),
        ObservationMode::Active => 0.85,
    };
    let roll = (signed_unit(seed, channel) + 1.0) * 0.5;
    if roll < reliability {
        actual
    } else if roll < reliability + (1.0 - reliability) * 0.6 {
        AcousticProfile::Mixed
    } else {
        match actual {
            AcousticProfile::LowFrequency => AcousticProfile::HighFrequency,
            AcousticProfile::HighFrequency => AcousticProfile::LowFrequency,
            AcousticProfile::Mixed => AcousticProfile::Mixed,
        }
    }
}
