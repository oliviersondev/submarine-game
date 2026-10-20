use shared::{AcousticProfile, SubmarineState};

use crate::KNOTS_TO_METERS_PER_SECOND;
use crate::{
    detection,
    enemy_ai::{EscortAi, Phase},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VesselKind {
    Cargo,
    Escort,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Vessel {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) heading: f32,
    pub(crate) speed: f32,
    pub(crate) signature: f32,
    integrity: f32,
    kind: VesselKind,
}

impl Vessel {
    pub(crate) fn afloat(&self) -> bool {
        self.integrity > 0.0
    }
    pub(crate) fn profile(&self) -> AcousticProfile {
        match self.kind {
            VesselKind::Cargo => AcousticProfile::LowFrequency,
            VesselKind::Escort => AcousticProfile::HighFrequency,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct World {
    seed: u64,
    pub(crate) vessels: Vec<Vessel>,
    pub(crate) escort_ai: EscortAi,
    escort_config: shared::EscortConfig,
}

impl World {
    pub(crate) fn new(seed: u64) -> Self {
        let offset = signed_unit(seed, 1) * 400.0;
        let heading = 165.0 + signed_unit(seed, 2) * 8.0;
        Self {
            seed,
            vessels: vec![
                vessel(VesselKind::Cargo, -550.0 + offset, 4_600.0, heading, 8.0),
                vessel(VesselKind::Cargo, 550.0 + offset, 4_850.0, heading, 7.5),
                vessel(VesselKind::Escort, offset, 4_050.0, heading, 11.0),
            ],
            escort_ai: EscortAi::default(),
            escort_config: shared::EscortConfig::default(),
        }
    }

    pub(crate) fn update(
        &mut self,
        dt: f32,
        submarine: &SubmarineState,
        tick: u64,
        config: shared::EscortConfig,
    ) -> Vec<shared::GameEvent> {
        self.escort_config = config;
        let mut events = Vec::new();
        if let Some(index) = self
            .vessels
            .iter()
            .position(|v| v.kind == VesselKind::Escort && v.afloat())
        {
            self.escort_ai.advance(dt, config);
            let vessel = &self.vessels[index];
            if self.escort_ai.passive_remaining <= 0.0 {
                self.escort_ai.passive_remaining = config.passive_interval.max(dt);
                if let Some(observation) =
                    detection::enemy_passive(vessel, submarine, config, self.seed, tick)
                {
                    self.escort_ai.observe(observation);
                }
            }
            if self.escort_ai.phase == Phase::Search && self.escort_ai.ping_remaining <= 0.0 {
                self.escort_ai.ping_remaining = config.ping_interval.max(dt);
                if let Some(event) =
                    detection::audible_enemy_ping(vessel, submarine, config, self.seed, tick)
                {
                    events.push(event);
                }
                if let Some(observation) =
                    detection::enemy_active_echo(vessel, submarine, config, self.seed, tick)
                {
                    self.escort_ai.observe(observation);
                }
            }
            let cargos: Vec<_> = self
                .vessels
                .iter()
                .filter(|v| v.kind == VesselKind::Cargo && v.afloat())
                .collect();
            let formation = if cargos.is_empty() {
                // With no convoy left, patrol the original area rather than follow the player.
                (0.0, 4_000.0, 165.0, 8.0)
            } else {
                let count = cargos.len() as f32;
                let angle = cargos[0].heading.to_radians();
                (
                    cargos.iter().map(|v| v.x).sum::<f32>() / count + angle.sin() * 675.0,
                    cargos.iter().map(|v| v.y).sum::<f32>() / count + angle.cos() * 675.0,
                    cargos[0].heading,
                    cargos[0].speed,
                )
            };
            let (heading, speed) = self
                .escort_ai
                .course((vessel.x, vessel.y), formation, config);
            let vessel = &mut self.vessels[index];
            let delta = (heading - vessel.heading + 180.0).rem_euclid(360.0) - 180.0;
            vessel.heading = (vessel.heading
                + delta.clamp(-config.turn_rate * dt, config.turn_rate * dt))
            .rem_euclid(360.0);
            vessel.speed +=
                (speed - vessel.speed).clamp(-config.acceleration * dt, config.acceleration * dt);
        } else {
            self.escort_ai = EscortAi::default();
        }
        for vessel in &mut self.vessels {
            if vessel.integrity <= 0.0 {
                continue;
            }
            let distance = vessel.speed * KNOTS_TO_METERS_PER_SECOND * dt;
            let heading = vessel.heading.to_radians();
            vessel.x += heading.sin() * distance;
            vessel.y += heading.cos() * distance;

            if vessel.kind != VesselKind::Escort
                && (vessel.x.abs() > 7_000.0 || vessel.y.abs() > 7_000.0)
            {
                vessel.heading = (vessel.heading + 180.0).rem_euclid(360.0);
            }
        }
        events
    }

    pub(crate) fn observe_active_ping(
        &mut self,
        submarine: &SubmarineState,
        tick: u64,
        config: shared::EscortConfig,
    ) {
        if let Some(vessel) = self
            .vessels
            .iter()
            .find(|v| v.kind == VesselKind::Escort && v.afloat())
        {
            if let Some(observation) =
                detection::enemy_hears_player_ping(vessel, submarine, config, self.seed, tick)
            {
                self.escort_ai.observe(observation);
            }
        }
    }

    pub(crate) fn impact(
        &mut self,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
        depth: f32,
        config: shared::WeaponsConfig,
    ) -> Option<bool> {
        if depth > config.max_surface_impact_depth {
            return None;
        }
        let dx = bx - ax;
        let dy = by - ay;
        let length = dx * dx + dy * dy;
        let hit = self.vessels.iter().position(|v| {
            if v.integrity <= 0.0 {
                return false;
            }
            let t = if length > 0.0 {
                (((v.x - ax) * dx + (v.y - ay) * dy) / length).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (v.x - ax - t * dx).hypot(v.y - ay - t * dy) <= config.collision_radius_meters
        })?;
        self.vessels[hit].integrity = (self.vessels[hit].integrity - config.damage).max(0.0);
        let sunk = self.vessels[hit].integrity == 0.0;
        if sunk {
            self.vessels[hit].signature = 0.0;
            self.vessels[hit].speed = 0.0;
        }
        // Les bâtiments voisins observent l'explosion, sans connaître le tireur.
        let (x, y) = (self.vessels[hit].x, self.vessels[hit].y);
        if self.vessels.iter().any(|v| {
            v.kind == VesselKind::Escort
                && v.afloat()
                && (v.x - x).hypot(v.y - y) < self.escort_config.explosion_hearing_range
        }) {
            self.escort_ai.alert(x, y);
        }
        for vessel in &mut self.vessels {
            if vessel.kind == VesselKind::Cargo
                && vessel.integrity > 0.0
                && (vessel.x - x).hypot(vessel.y - y) < 3000.0
            {
                vessel.heading = (vessel.heading + 35.0).rem_euclid(360.0);
            }
        }
        Some(sunk)
    }

    #[cfg(test)]
    pub(crate) fn enemy_track(&self) -> Option<(f32, f32, f32, u64)> {
        self.escort_ai
            .track
            .as_ref()
            .map(|track| (track.x, track.y, track.uncertainty, track.observed_tick))
    }
}

fn vessel(kind: VesselKind, x: f32, y: f32, heading: f32, speed: f32) -> Vessel {
    Vessel {
        x,
        y,
        heading,
        speed,
        signature: match kind {
            VesselKind::Cargo => 82.0,
            VesselKind::Escort => 72.0,
        },
        integrity: 100.0,
        kind,
    }
}

pub(crate) fn signed_unit(seed: u64, channel: u64) -> f32 {
    let mut value = seed ^ channel.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    (value as u32 as f32 / u32::MAX as f32) * 2.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{EscortConfig, GameEvent, WeaponsConfig};

    fn silent_sub(x: f32, y: f32) -> SubmarineState {
        SubmarineState {
            x,
            y,
            depth: 50.0,
            acoustic_signature: 0.0,
            ..SubmarineState::default()
        }
    }

    #[test]
    fn ping_hearing_requires_range_and_a_live_escort() {
        let config = EscortConfig::default();
        let mut world = World::new(0);
        let far = silent_sub(50_000.0, 50_000.0);
        world.observe_active_ping(&far, 1, config);
        assert!(world.enemy_track().is_none());
        let near = silent_sub(0.0, 0.0);
        world.observe_active_ping(&near, 2, config);
        assert!(world.enemy_track().is_some());
        world.vessels[2].integrity = 0.0;
        world.update(0.05, &near, 3, config);
        world.observe_active_ping(&near, 4, config);
        assert!(world.enemy_track().is_none());
        assert_eq!(world.escort_ai.phase, Phase::Escort);
        assert!(world.update(20.0, &near, 5, config).is_empty());
    }

    #[test]
    fn nearby_noise_attracts_escort_but_silence_does_not() {
        let config = EscortConfig::default();
        let mut noisy_world = World::new(0);
        let mut quiet_world = noisy_world.clone();
        let escort = &noisy_world.vessels[2];
        let quiet = silent_sub(escort.x + 500.0, escort.y);
        let noisy = SubmarineState {
            acoustic_signature: 100.0,
            ..quiet.clone()
        };
        for tick in 1..=120 {
            noisy_world.update(0.05, &noisy, tick, config);
            quiet_world.update(0.05, &quiet, tick, config);
        }
        assert_eq!(noisy_world.escort_ai.phase, Phase::Search);
        assert!(noisy_world.enemy_track().is_some());
        assert_eq!(quiet_world.escort_ai.phase, Phase::Escort);
        assert!(quiet_world.enemy_track().is_none());
    }

    #[test]
    fn hearing_an_enemy_ping_does_not_imply_an_echo() {
        let config = EscortConfig::default();
        let mut world = World::new(0);
        let submarine = silent_sub(0.0, 0.0);
        world.observe_active_ping(&submarine, 0, config);
        let initial_tick = world.enemy_track().unwrap().3;
        let events = world.update(0.05, &submarine, 1, config);
        assert!(matches!(
            events.as_slice(),
            [GameEvent::SonarPingHeard { .. }]
        ));
        // The escort is over 4 km away: audible emission, but outside echo range.
        assert_eq!(world.enemy_track().unwrap().3, initial_tick);
        assert!(world.update(0.05, &submarine, 2, config).is_empty());
    }

    #[test]
    fn active_echo_is_imperfect_and_refreshes_only_inside_range() {
        let config = EscortConfig::default();
        let mut world = World::new(0);
        let vessel = &world.vessels[2];
        let submarine = silent_sub(vessel.x + 1_000.0, vessel.y);
        world.observe_active_ping(&submarine, 0, config);
        world.update(0.05, &submarine, 1, config);
        let (x, y, uncertainty, tick) = world.enemy_track().unwrap();
        assert_eq!(tick, 1);
        assert!(uncertainty > 0.0);
        assert_ne!((x, y), (submarine.x, submarine.y));
    }

    #[test]
    fn unheard_emission_has_no_public_event_even_while_searching() {
        let config = EscortConfig::default();
        let mut world = World::new(0);
        world.observe_active_ping(&silent_sub(0.0, 0.0), 0, config);
        assert!(world
            .update(0.05, &silent_sub(50_000.0, 50_000.0), 1, config)
            .is_empty());
        assert!(world.escort_ai.ping_remaining > 0.0);
    }

    #[test]
    fn hidden_player_moves_do_not_change_search_and_escort_eventually_returns() {
        let config = EscortConfig::default();
        let mut a = World::new(42);
        a.observe_active_ping(&silent_sub(5_000.0, 4_000.0), 0, config);
        let mut b = a.clone();
        let mut search_seen = false;
        for tick in 1..=12_000 {
            a.update(0.05, &silent_sub(30_000.0, 20_000.0), tick, config);
            b.update(0.05, &silent_sub(-40_000.0, -30_000.0), tick, config);
            assert_eq!(
                a, b,
                "unobserved player moves changed private world at tick {tick}"
            );
            search_seen |= a.escort_ai.phase == Phase::Search;
        }
        assert!(search_seen);
        assert!(a.enemy_track().is_none());
        assert_eq!(a.escort_ai.phase, Phase::Escort);
    }

    #[test]
    fn escort_motion_is_bounded_including_after_an_explosion() {
        let config = EscortConfig::default();
        let mut world = World::new(0);
        let (x, y) = (world.vessels[0].x, world.vessels[0].y);
        let old_heading = world.vessels[2].heading;
        world.impact(x, y, x, y, 2.0, WeaponsConfig::default());
        assert_eq!(world.vessels[2].heading, old_heading);
        assert!(world.enemy_track().is_none());
        assert_eq!(world.escort_ai.phase, Phase::Suspicion);
        let old = world.vessels[2].clone();
        world.update(0.05, &silent_sub(50_000.0, 50_000.0), 1, config);
        let new = &world.vessels[2];
        let turn = (new.heading - old.heading + 180.0).rem_euclid(360.0) - 180.0;
        assert!(turn.abs() <= config.turn_rate * 0.05 + 0.0001);
        assert!((new.speed - old.speed).abs() <= config.acceleration * 0.05 + 0.0001);
    }

    #[test]
    fn distant_explosion_does_not_alert_escort() {
        let mut world = World::new(0);
        world.vessels[2].x = 30_000.0;
        let (x, y) = (world.vessels[0].x, world.vessels[0].y);
        world.impact(x, y, x, y, 2.0, WeaponsConfig::default());
        assert_eq!(world.escort_ai.phase, Phase::Escort);
        assert!(world.enemy_track().is_none());
    }
}
