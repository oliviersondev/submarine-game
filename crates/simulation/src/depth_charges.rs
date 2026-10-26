//! Unguided charges: after release, no input from target state or enemy sensors.
use shared::EscortAttackConfig;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Charge {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) depth: f32,
    fuse_depth: f32,
    age: f32,
    config: EscortAttackConfig,
}

impl Charge {
    pub(crate) fn salvo(x: f32, y: f32, heading: f32, config: EscortAttackConfig) -> Vec<Self> {
        (-1..=1)
            .map(|i| {
                let lateral = i as f32 * 25.0;
                Self {
                    x: x + heading.to_radians().cos() * lateral,
                    y: y - heading.to_radians().sin() * lateral,
                    depth: 0.0,
                    fuse_depth: (config.depth + i as f32 * config.depth_spacing).clamp(5.0, 250.0),
                    age: 0.0,
                    config,
                }
            })
            .collect()
    }

    pub(crate) fn advance(&mut self, dt: f32) -> bool {
        self.age += dt;
        self.depth = (self.depth + self.config.sink_speed.max(0.1) * dt).min(self.fuse_depth);
        self.depth >= self.fuse_depth && self.age >= self.config.arming_time
    }

    pub(crate) fn distance(&self, x: f32, y: f32, depth: f32) -> f32 {
        (self.x - x).hypot(self.y - y).hypot(self.depth - depth)
    }

    pub(crate) fn damage_at(&self, distance: f32) -> f32 {
        self.config.damage * (1.0 - distance / self.config.blast_radius.max(0.1)).clamp(0.0, 1.0)
    }

    pub(crate) fn audible_at(&self, distance: f32) -> bool {
        distance <= self.config.hearing_range
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charges_sink_to_preset_depth_without_horizontal_homing() {
        let mut charges = Charge::salvo(100.0, 200.0, 0.0, EscortAttackConfig::default());
        let charge = &mut charges[1];
        assert!(!charge.advance(19.0));
        assert_eq!(charge.depth, 95.0);
        assert!(charge.advance(1.0));
        assert_eq!((charge.x, charge.y, charge.depth), (100.0, 200.0, 100.0));
    }

    #[test]
    fn shallow_charge_still_waits_for_arming_time() {
        let config = EscortAttackConfig {
            depth: 5.0,
            depth_spacing: 0.0,
            sink_speed: 100.0,
            ..EscortAttackConfig::default()
        };
        let mut charge = Charge::salvo(0.0, 0.0, 0.0, config).remove(1);
        assert!(!charge.advance(1.0));
        assert_eq!(charge.depth, 5.0);
        assert!(!charge.advance(1.0));
        assert!(charge.advance(1.0));
    }

    #[test]
    fn damage_uses_slant_distance_and_moving_away_can_avoid_it() {
        let mut charge = Charge::salvo(0.0, 0.0, 0.0, EscortAttackConfig::default()).remove(1);
        assert!(charge.advance(20.0));
        assert_eq!(charge.damage_at(charge.distance(0.0, 0.0, 100.0)), 24.0);
        assert_eq!(charge.damage_at(charge.distance(121.0, 0.0, 100.0)), 0.0);
        assert_eq!(charge.damage_at(charge.distance(0.0, 0.0, 221.0)), 0.0);
        assert!(charge.audible_at(2_000.0));
        assert!(!charge.audible_at(3_001.0));
    }
}
