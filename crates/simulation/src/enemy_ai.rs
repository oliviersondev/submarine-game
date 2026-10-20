//! Private escort decisions. This module never receives the player's true state.
use shared::EscortConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    Escort,
    Suspicion,
    Search,
    Returning,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EnemyTrack {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) uncertainty: f32,
    pub(crate) confidence: f32,
    pub(crate) observed_tick: u64,
    age: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Observation {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) uncertainty: f32,
    pub(crate) confidence: f32,
    pub(crate) tick: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EscortAi {
    pub(crate) phase: Phase,
    pub(crate) track: Option<EnemyTrack>,
    // An explosion indicates danger here, not the location of the submarine.
    alert_zone: Option<(f32, f32)>,
    phase_age: f32,
    return_grace: f32,
    search_leg: u32,
    leg_age: f32,
    pub(crate) passive_remaining: f32,
    pub(crate) ping_remaining: f32,
}

impl Default for EscortAi {
    fn default() -> Self {
        Self {
            phase: Phase::Escort,
            track: None,
            alert_zone: None,
            phase_age: 0.0,
            return_grace: 0.0,
            search_leg: 0,
            leg_age: 0.0,
            passive_remaining: 0.0,
            ping_remaining: 0.0,
        }
    }
}

impl EscortAi {
    pub(crate) fn observe(&mut self, observation: Observation) {
        if self.return_grace > 0.0 {
            return;
        }
        let confidence = self.track.as_ref().map_or(0.0, |t| t.confidence);
        self.track = Some(EnemyTrack {
            x: observation.x,
            y: observation.y,
            uncertainty: observation.uncertainty,
            confidence: (confidence + observation.confidence).min(100.0),
            observed_tick: observation.tick,
            age: 0.0,
        });
        if matches!(self.phase, Phase::Escort | Phase::Returning) {
            self.enter(Phase::Suspicion);
        }
    }

    pub(crate) fn alert(&mut self, x: f32, y: f32) {
        if self.return_grace <= 0.0 && self.phase != Phase::Search {
            self.alert_zone = Some((x, y));
            self.enter(Phase::Suspicion);
        }
    }

    fn enter(&mut self, phase: Phase) {
        self.phase = phase;
        self.phase_age = 0.0;
        self.leg_age = 0.0;
        self.search_leg = 0;
    }

    pub(crate) fn advance(&mut self, dt: f32, config: EscortConfig) {
        self.phase_age += dt;
        self.leg_age += dt;
        self.passive_remaining = (self.passive_remaining - dt).max(0.0);
        self.ping_remaining = (self.ping_remaining - dt).max(0.0);
        self.return_grace = (self.return_grace - dt).max(0.0);
        if let Some(track) = &mut self.track {
            track.age += dt;
            track.confidence = (track.confidence - config.confidence_decay * dt).max(0.0);
            track.uncertainty = (track.uncertainty + config.uncertainty_growth * dt).min(4_000.0);
            if track.age >= config.track_lifetime || track.confidence <= 0.0 {
                self.track = None;
            }
        }
        match self.phase {
            Phase::Suspicion => {
                if self
                    .track
                    .as_ref()
                    .is_some_and(|t| t.confidence >= config.search_confidence)
                {
                    self.enter(Phase::Search);
                } else if self.phase_age >= config.suspicion_duration {
                    // A convoy alarm permits a search around the explosion only.
                    if self.alert_zone.is_some() {
                        self.enter(Phase::Search);
                    } else {
                        self.abandon(config);
                    }
                }
            }
            Phase::Search => {
                if self.phase_age >= config.search_duration
                    || (self.track.is_none() && self.alert_zone.is_none())
                {
                    self.abandon(config);
                }
            }
            _ => {}
        }
    }

    fn abandon(&mut self, config: EscortConfig) {
        self.track = None;
        self.alert_zone = None;
        self.return_grace = config.return_grace;
        self.enter(Phase::Returning);
    }

    /// Own position and friendly formation are known; target position is estimated.
    pub(crate) fn course(
        &mut self,
        own: (f32, f32),
        formation: (f32, f32, f32, f32),
        config: EscortConfig,
    ) -> (f32, f32) {
        let (fx, fy, heading, speed) = formation;
        if matches!(self.phase, Phase::Escort | Phase::Returning) {
            let distance = (fx - own.0).hypot(fy - own.1);
            if distance < 250.0 {
                if self.phase == Phase::Returning {
                    self.enter(Phase::Escort);
                }
                return (heading, speed);
            }
            return (bearing(own, (fx, fy)), config.return_speed);
        }
        let (x, y, radius) = self.track.as_ref().map_or_else(
            || {
                let (x, y) = self.alert_zone.unwrap_or((own.0, own.1));
                (x, y, 600.0)
            },
            |t| (t.x, t.y, t.uncertainty.max(150.0)),
        );
        let angle = (self.search_leg as f32 * 137.5).to_radians();
        let target = if self.search_leg == 0 {
            (x, y)
        } else {
            (x + angle.sin() * radius, y + angle.cos() * radius)
        };
        if (target.0 - own.0).hypot(target.1 - own.1) < 150.0 || self.leg_age >= 35.0 {
            self.search_leg = self.search_leg.wrapping_add(1);
            self.leg_age = 0.0;
        }
        (bearing(own, target), config.search_speed)
    }
}

fn bearing(from: (f32, f32), to: (f32, f32)) -> f32 {
    (to.0 - from.0)
        .atan2(to.1 - from.1)
        .to_degrees()
        .rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation() -> Observation {
        Observation {
            x: 500.0,
            y: 800.0,
            uncertainty: 250.0,
            confidence: 60.0,
            tick: 1,
        }
    }

    #[test]
    fn observations_trigger_search_then_age_out_without_revealing_an_absence() {
        let config = EscortConfig::default();
        let mut ai = EscortAi::default();
        ai.observe(observation());
        assert_eq!(ai.phase, Phase::Suspicion);
        ai.advance(0.05, config);
        assert_eq!(ai.phase, Phase::Search);
        ai.advance(10.0, config);
        let track = ai.track.as_ref().unwrap();
        assert!(track.confidence < 60.0);
        assert!(track.uncertainty > 250.0);
        assert_eq!((track.x, track.y), (500.0, 800.0));
        ai.advance(config.track_lifetime, config);
        assert!(ai.track.is_none());
        assert_eq!(ai.phase, Phase::Returning);
        // A short grace period prevents immediate re-entry after abandoning.
        ai.observe(observation());
        assert!(ai.track.is_none());
        ai.advance(config.return_grace, config);
        ai.observe(observation());
        assert_eq!(ai.phase, Phase::Suspicion);
    }

    #[test]
    fn explosion_alert_is_not_a_submarine_track_and_search_is_bounded() {
        let config = EscortConfig::default();
        let mut ai = EscortAi::default();
        ai.alert(100.0, 200.0);
        assert!(ai.track.is_none());
        ai.advance(config.suspicion_duration, config);
        assert_eq!(ai.phase, Phase::Search);
        ai.advance(config.search_duration, config);
        assert_eq!(ai.phase, Phase::Returning);
        assert!(ai.alert_zone.is_none());
        let formation = (0.0, 0.0, 165.0, 8.0);
        assert_eq!(ai.course((0.0, 0.0), formation, config), (165.0, 8.0));
        assert_eq!(ai.phase, Phase::Escort);
    }

    #[test]
    fn weak_unconfirmed_contact_does_not_cause_an_endless_search() {
        let config = EscortConfig::default();
        let mut ai = EscortAi::default();
        ai.observe(Observation {
            confidence: 18.0,
            ..observation()
        });
        ai.advance(config.suspicion_duration, config);
        assert_eq!(ai.phase, Phase::Returning);
        assert!(ai.track.is_none());
    }

    #[test]
    fn repeated_observations_do_not_reset_the_maximum_search_duration() {
        let config = EscortConfig {
            search_duration: 2.0,
            ..EscortConfig::default()
        };
        let mut ai = EscortAi::default();
        ai.observe(observation());
        ai.advance(0.05, config);
        for _ in 0..3 {
            ai.observe(observation());
            ai.advance(1.0, config);
        }
        assert_eq!(ai.phase, Phase::Returning);
    }
}
