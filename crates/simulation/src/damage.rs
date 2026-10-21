//! Authoritative persistent damage. No enemy decisions or networking here.
use shared::{CompartmentState, DamageCause, DamageConfig, DamageRecord, DamageSummary, GameEvent};
use std::collections::VecDeque;

pub(crate) const HISTORY_LIMIT: usize = 64;

pub(crate) struct Damage {
    pub(crate) compartments: [CompartmentState; 3],
    pub(crate) history: VecDeque<DamageRecord>,
    destroyed: bool,
    pressure_elapsed: f32,
}

impl Default for Damage {
    fn default() -> Self {
        Self {
            compartments: [CompartmentState::default(); 3],
            history: VecDeque::new(),
            destroyed: false,
            pressure_elapsed: 0.0,
        }
    }
}

impl Damage {
    pub(crate) fn summary(&self, hull: f32) -> DamageSummary {
        DamageSummary {
            hull_integrity: hull,
            flooding: self.flooding(),
            destroyed: self.destroyed,
            last_cause: self.history.back().map(|record| record.cause),
        }
    }

    pub(crate) fn flooding(&self) -> f32 {
        self.compartments.iter().map(|c| c.flooding).sum::<f32>() / 3.0
    }

    pub(crate) fn hit(
        &mut self,
        hull: &mut f32,
        amount: f32,
        compartment: usize,
        tick: u64,
        config: DamageConfig,
    ) -> Vec<GameEvent> {
        if self.destroyed || !amount.is_finite() || amount <= 0.0 {
            return vec![];
        }
        let c = &mut self.compartments[compartment];
        c.integrity = (c.integrity - amount).max(0.0);
        c.breach_rate = (c.breach_rate + amount * config.breach_per_damage).clamp(0.0, 1.0);
        self.apply(
            hull,
            amount,
            DamageCause::DepthCharge,
            Some(compartment as u8),
            tick,
        )
    }

    fn apply(
        &mut self,
        hull: &mut f32,
        amount: f32,
        cause: DamageCause,
        compartment: Option<u8>,
        tick: u64,
    ) -> Vec<GameEvent> {
        let previous = *hull;
        *hull = (*hull - amount).clamp(0.0, 100.0);
        self.destroyed = *hull <= 0.0 || self.flooding() >= 100.0;
        self.history.push_back(DamageRecord {
            tick,
            cause,
            hull_damage: previous - *hull,
            compartment,
            destroyed: self.destroyed,
        });
        if self.history.len() > HISTORY_LIMIT {
            self.history.pop_front();
        }
        let mut events = vec![GameEvent::SubmarineDamaged { cause }];
        if self.destroyed {
            events.push(GameEvent::SubmarineDestroyed { cause });
        }
        events
    }

    pub(crate) fn advance(
        &mut self,
        hull: &mut f32,
        dt: f32,
        depth: f32,
        crush_depth: f32,
        tick: u64,
        config: DamageConfig,
    ) -> Vec<GameEvent> {
        if self.destroyed {
            return vec![];
        }
        for compartment in &mut self.compartments {
            // Pressure accelerates ingress; no implicit pumping or repair in M5.2.
            let pressure = 1.0 + depth.max(0.0) / crush_depth.max(1.0);
            compartment.flooding =
                (compartment.flooding + compartment.breach_rate * pressure * dt).clamp(0.0, 100.0);
        }
        if self.flooding() >= 100.0 {
            return self.apply(hull, *hull, DamageCause::Flooding, None, tick);
        }
        if depth >= crush_depth {
            self.pressure_elapsed += dt;
            if self.pressure_elapsed >= 1.0 {
                let seconds = self.pressure_elapsed.floor();
                self.pressure_elapsed -= seconds;
                return self.apply(
                    hull,
                    config.crush_damage_per_second * seconds,
                    DamageCause::Pressure,
                    None,
                    tick,
                );
            }
        } else {
            self.pressure_elapsed = 0.0;
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breach_persists_and_flooding_is_bounded() {
        let mut damage = Damage::default();
        let mut hull = 100.0;
        damage.hit(&mut hull, 20.0, 0, 1, DamageConfig::default());
        damage.advance(&mut hull, 10.0, 100.0, 250.0, 2, DamageConfig::default());
        let first = damage.flooding();
        assert!(first > 0.0);
        damage.advance(&mut hull, 10.0, 100.0, 250.0, 3, DamageConfig::default());
        assert!(damage.flooding() > first);
        damage.advance(
            &mut hull,
            100_000.0,
            100.0,
            250.0,
            4,
            DamageConfig::default(),
        );
        assert_eq!(damage.compartments[0].flooding, 100.0);
        assert_eq!(damage.compartments[1].flooding, 0.0);
    }

    #[test]
    fn destruction_emits_once_and_keeps_its_cause() {
        let mut damage = Damage::default();
        let mut hull = 10.0;
        let events = damage.hit(&mut hull, 24.0, 2, 42, DamageConfig::default());
        assert_eq!(hull, 0.0);
        assert_eq!(
            events.last(),
            Some(&GameEvent::SubmarineDestroyed {
                cause: DamageCause::DepthCharge
            })
        );
        assert!(damage
            .hit(&mut hull, 24.0, 2, 43, DamageConfig::default())
            .is_empty());
        assert!(damage.summary(hull).destroyed);
        assert!(damage.history.back().unwrap().destroyed);
    }

    #[test]
    fn pressure_and_flooding_have_distinct_persistent_causes() {
        let mut damage = Damage::default();
        let mut hull = 1.0;
        damage.advance(&mut hull, 1.0, 250.0, 250.0, 20, DamageConfig::default());
        assert_eq!(damage.history.back().unwrap().cause, DamageCause::Pressure);
        let mut damage = Damage::default();
        let mut hull = 100.0;
        for c in &mut damage.compartments {
            c.flooding = 100.0;
        }
        damage.advance(&mut hull, 0.05, 50.0, 250.0, 21, DamageConfig::default());
        assert_eq!(damage.history.back().unwrap().cause, DamageCause::Flooding);
        assert_eq!(hull, 0.0);
    }

    #[test]
    fn history_is_bounded() {
        let mut damage = Damage::default();
        let mut hull = 100.0;
        for tick in 0..100 {
            damage.hit(&mut hull, 0.01, 0, tick, DamageConfig::default());
        }
        assert_eq!(damage.history.len(), HISTORY_LIMIT);
        assert_eq!(damage.history.front().unwrap().tick, 36);
    }
}
