//! Enemy visual sensor boundary. Only completed observations reach the escort AI.
use shared::{LookoutConfig, PeriscopeDetectionConfig, SubmarineState};

use crate::{enemy_ai::Observation, world::signed_unit};

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Lookout {
    progress: f32,
}

impl Lookout {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update(
        &mut self,
        dt: f32,
        own: (f32, f32),
        submarine: &SubmarineState,
        exposed: bool,
        profile: LookoutConfig,
        config: PeriscopeDetectionConfig,
        seed: u64,
        tick: u64,
        observer: u64,
    ) -> Option<Observation> {
        if !dt.is_finite() || dt <= 0.0 {
            return None;
        }
        // Invalid sensor tuning disables acquisition rather than producing NaNs.
        if !valid(profile, config) {
            self.progress = 0.0;
            return None;
        }
        let dx = submarine.x - own.0;
        let dy = submarine.y - own.1;
        let distance = dx.hypot(dy);
        if !exposed
            || !distance.is_finite()
            || distance > profile.range
            || !submarine.speed.is_finite()
        {
            self.progress = (self.progress - dt / config.memory_time).max(0.0);
            return None;
        }
        let proximity =
            1.0 + (config.proximity_multiplier - 1.0) * (1.0 - distance / profile.range);
        let speed = 1.0 + submarine.speed.abs() * config.speed_factor;
        self.progress += dt / profile.acquisition_time * proximity * speed;
        if self.progress < 1.0 {
            return None;
        }
        self.progress = 0.0;
        // Separate deterministic channels for each observer and from acoustics.
        let channel = tick
            .wrapping_mul(64)
            .wrapping_add(observer.wrapping_mul(2))
            .wrapping_add(32);
        let angle =
            dx.atan2(dy) + (signed_unit(seed, channel) * profile.bearing_error).to_radians();
        let uncertainty = 100.0 + distance * profile.range_error;
        let range = (distance + signed_unit(seed, channel + 1) * uncertainty).max(0.0);
        let observation = Observation {
            x: own.0 + angle.sin() * range,
            y: own.1 + angle.cos() * range,
            uncertainty: uncertainty + distance * profile.bearing_error.to_radians(),
            confidence: profile.confidence,
            tick,
        };
        // Even extreme finite tuning must not feed non-finite estimates to the AI.
        [observation.x, observation.y, observation.uncertainty]
            .iter()
            .all(|value| value.is_finite())
            .then_some(observation)
    }
}

fn valid(profile: LookoutConfig, config: PeriscopeDetectionConfig) -> bool {
    [profile.range, profile.acquisition_time, config.memory_time]
        .iter()
        .all(|v| v.is_finite() && *v > 0.0)
        && [
            profile.bearing_error,
            profile.range_error,
            config.speed_factor,
        ]
        .iter()
        .all(|v| v.is_finite() && *v >= 0.0)
        && config.proximity_multiplier.is_finite()
        && config.proximity_multiplier >= 1.0
        && profile.confidence.is_finite()
        && (0.0..=100.0).contains(&profile.confidence)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn submarine(distance: f32, speed: f32) -> SubmarineState {
        SubmarineState {
            x: distance,
            y: 0.0,
            speed,
            ..SubmarineState::default()
        }
    }

    fn first_detection(profile: LookoutConfig, distance: f32, speed: f32) -> u64 {
        let mut lookout = Lookout::default();
        for tick in 1..=2_000 {
            if lookout
                .update(
                    0.05,
                    (0.0, 0.0),
                    &submarine(distance, speed),
                    true,
                    profile,
                    PeriscopeDetectionConfig::default(),
                    42,
                    tick,
                    0,
                )
                .is_some()
            {
                return tick;
            }
        }
        panic!("no visual detection");
    }

    #[test]
    fn acquisition_is_progressive_and_faster_nearby_or_at_speed() {
        let profile = PeriscopeDetectionConfig::default().escort;
        let baseline = first_detection(profile, 2_000.0, 0.0);
        assert!(baseline > 1);
        assert!(first_detection(profile, 500.0, 0.0) < baseline);
        assert!(first_detection(profile, 2_000.0, 6.0) < baseline);
    }

    #[test]
    fn cargos_detect_later_with_shorter_range_and_worse_measurements() {
        let c = PeriscopeDetectionConfig::default();
        assert!(c.cargo.range < c.escort.range);
        assert!(first_detection(c.cargo, 1_000.0, 0.0) > first_detection(c.escort, 1_000.0, 0.0));
        assert!(c.cargo.bearing_error > c.escort.bearing_error);
        assert!(c.cargo.range_error > c.escort.range_error);
        assert!(c.cargo.confidence < c.escort.confidence);
    }

    #[test]
    fn lowered_out_of_range_and_invalid_values_never_produce_observations() {
        let c = PeriscopeDetectionConfig::default();
        for (exposed, distance, speed) in [
            (false, 0.0, 0.0),
            (true, c.escort.range + 0.1, 0.0),
            (true, f32::NAN, 0.0),
            (true, 0.0, f32::INFINITY),
        ] {
            let mut lookout = Lookout::default();
            for tick in 0..1_000 {
                assert!(lookout
                    .update(
                        0.05,
                        (0.0, 0.0),
                        &submarine(distance, speed),
                        exposed,
                        c.escort,
                        c,
                        42,
                        tick,
                        0
                    )
                    .is_none());
            }
            assert_eq!(lookout.progress, 0.0);
        }
        let mut lookout = Lookout::default();
        for dt in [f32::NAN, f32::INFINITY, 0.0, -1.0] {
            assert!(lookout
                .update(
                    dt,
                    (0.0, 0.0),
                    &submarine(0.0, 0.0),
                    true,
                    c.escort,
                    c,
                    42,
                    0,
                    0
                )
                .is_none());
        }
        let bad = LookoutConfig {
            acquisition_time: 0.0,
            ..c.escort
        };
        assert!(lookout
            .update(
                1.0,
                (0.0, 0.0),
                &submarine(0.0, 0.0),
                true,
                bad,
                c,
                42,
                0,
                0
            )
            .is_none());
        assert_eq!(lookout.progress, 0.0);
    }

    #[test]
    fn brief_lowering_decays_but_does_not_reset_acquisition() {
        let c = PeriscopeDetectionConfig::default();
        let sub = submarine(c.escort.range, 0.0);
        let mut lookout = Lookout::default();
        assert!(lookout
            .update(6.0, (0.0, 0.0), &sub, true, c.escort, c, 42, 1, 0)
            .is_none());
        assert_eq!(lookout.progress, 0.5);
        lookout.update(1.0, (0.0, 0.0), &sub, false, c.escort, c, 42, 2, 0);
        assert!((lookout.progress - 0.4).abs() < 0.0001);
        assert!(lookout
            .update(7.3, (0.0, 0.0), &sub, true, c.escort, c, 42, 3, 0)
            .is_some());
        lookout.progress = 0.9;
        lookout.update(
            c.memory_time,
            (0.0, 0.0),
            &sub,
            false,
            c.escort,
            c,
            42,
            4,
            0,
        );
        assert_eq!(lookout.progress, 0.0);
    }

    #[test]
    fn range_boundary_is_inclusive_and_observations_are_imperfect_and_repeatable() {
        let c = PeriscopeDetectionConfig::default();
        let sub = submarine(c.escort.range, 0.0);
        let sample = |observer| {
            Lookout::default()
                .update(
                    c.escort.acquisition_time,
                    (0.0, 0.0),
                    &sub,
                    true,
                    c.escort,
                    c,
                    42,
                    1,
                    observer,
                )
                .unwrap()
        };
        let a = sample(0);
        let b = sample(0);
        assert_eq!((a.x, a.y, a.uncertainty), (b.x, b.y, b.uncertainty));
        assert_ne!((a.x, a.y), (sub.x, sub.y));
        assert!(a.uncertainty > 0.0);
        assert_ne!((a.x, a.y), (sample(1).x, sample(1).y));
    }
}
