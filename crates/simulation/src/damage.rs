//! Authoritative persistent damage. No enemy decisions or networking here.
use shared::{
    CompartmentState, DamageCause, DamageConfig, DamageControlAction, DamageRecord, DamageSummary,
    GameEvent, Intervention, PlayerCommand, ProtocolError, SystemId,
};
use std::collections::VecDeque;

pub(crate) const HISTORY_LIMIT: usize = 64;

pub(crate) struct Damage {
    pub(crate) compartments: [CompartmentState; 3],
    pub(crate) history: VecDeque<DamageRecord>,
    destroyed: bool,
    pressure_elapsed: f32,
    fire_elapsed: f32,
    failed: [bool; 5],
    pub(crate) intervention: Option<Intervention>,
    pumps_powered: bool,
}

impl Default for Damage {
    fn default() -> Self {
        Self {
            compartments: [CompartmentState::default(); 3],
            history: VecDeque::new(),
            destroyed: false,
            pressure_elapsed: 0.0,
            fire_elapsed: 0.0,
            failed: [false; 5],
            intervention: None,
            pumps_powered: false,
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
            fire: self.compartments.iter().map(|c| c.fire).sum::<f32>() / 3.0,
            unavailable_systems: SystemId::ALL
                .into_iter()
                .filter(|id| !self.available(*id))
                .count() as u8,
        }
    }

    pub(crate) fn flooding(&self) -> f32 {
        self.compartments.iter().map(|c| c.flooding).sum::<f32>() / 3.0
    }

    pub(crate) fn available(&self, system: SystemId) -> bool {
        let c = &self.compartments[system.compartment() as usize];
        !self.failed[system as usize] && !c.isolated && c.fire < 25.0 && c.flooding < 50.0
    }

    pub(crate) fn pumps(&self, powered: bool) -> usize {
        if powered {
            self.compartments.iter().filter(|c| c.pumping).count()
        } else {
            0
        }
    }

    pub(crate) fn faults(&self) -> Vec<SystemId> {
        SystemId::ALL
            .into_iter()
            .filter(|id| self.failed[*id as usize])
            .collect()
    }

    pub(crate) fn command(
        &mut self,
        command: PlayerCommand,
        battery: f32,
        config: DamageConfig,
    ) -> Result<(), ProtocolError> {
        if self.destroyed {
            return Err(ProtocolError::SubmarineDestroyed);
        }
        let (index, action) = match command {
            PlayerCommand::SetCompartmentPump {
                compartment,
                enabled,
            } => {
                let c = self
                    .compartments
                    .get_mut(compartment as usize)
                    .ok_or(ProtocolError::InvalidCompartment)?;
                if enabled && battery <= 0.0 {
                    return Err(ProtocolError::InsufficientBattery);
                }
                c.pumping = enabled;
                return Ok(());
            }
            PlayerCommand::SetCompartmentIsolated {
                compartment,
                isolated,
            } => {
                self.compartments
                    .get_mut(compartment as usize)
                    .ok_or(ProtocolError::InvalidCompartment)?
                    .isolated = isolated;
                return Ok(());
            }
            PlayerCommand::StartDamageControl {
                compartment,
                action,
            } => (compartment, action),
            PlayerCommand::RepairSystem(id) => {
                (id.compartment(), DamageControlAction::RepairSystem(id))
            }
            PlayerCommand::CancelDamageControl => {
                self.intervention = None;
                return Ok(());
            }
            _ => unreachable!("damage-control commands only"),
        };
        let c = self
            .compartments
            .get(index as usize)
            .ok_or(ProtocolError::InvalidCompartment)?;
        if self.intervention.is_some() {
            return Err(ProtocolError::DamageControlBusy);
        }
        let duration = match action {
            DamageControlAction::SealBreach => {
                if c.breach_rate <= 0.0 {
                    return Err(ProtocolError::NoDamageToRepair);
                }
                if c.fire >= 25.0 || c.flooding >= 90.0 {
                    return Err(ProtocolError::UnsafeCompartment);
                }
                config.seal_seconds
            }
            DamageControlAction::Extinguish => {
                if c.fire <= 0.0 {
                    return Err(ProtocolError::NoDamageToRepair);
                }
                config.extinguish_seconds
            }
            DamageControlAction::RepairSystem(id) => {
                if id.compartment() != index {
                    return Err(ProtocolError::InvalidCompartment);
                }
                if !self.failed[id as usize] {
                    return Err(ProtocolError::NoDamageToRepair);
                }
                if c.fire >= 25.0 || c.flooding >= 50.0 {
                    return Err(ProtocolError::UnsafeCompartment);
                }
                config.repair_seconds
            }
        }
        .max(0.05);
        self.intervention = Some(Intervention {
            compartment: index,
            action,
            remaining: duration,
            duration,
            paused: false,
        });
        Ok(())
    }

    /// One team. Unsafe work pauses rather than silently completing under water/fire.
    pub(crate) fn advance_control(&mut self, dt: f32, powered: bool) {
        if self.destroyed {
            self.intervention = None;
            return;
        }
        self.pumps_powered = powered;
        let Some(task) = &mut self.intervention else {
            return;
        };
        let c = &mut self.compartments[task.compartment as usize];
        let safe = match task.action {
            DamageControlAction::Extinguish => true,
            DamageControlAction::SealBreach => c.fire < 25.0 && c.flooding < 90.0,
            DamageControlAction::RepairSystem(_) => c.fire < 25.0 && c.flooding < 50.0,
        };
        task.paused = !safe;
        if !safe {
            return;
        }
        task.remaining = (task.remaining - dt).max(0.0);
        if task.remaining > 0.0 {
            return;
        }
        match task.action {
            DamageControlAction::SealBreach => c.breach_rate = 0.0,
            DamageControlAction::Extinguish => c.fire = 0.0,
            DamageControlAction::RepairSystem(id) => self.failed[id as usize] = false,
        }
        self.intervention = None;
    }

    pub(crate) fn bot_commands(&self, battery: f32) -> Vec<PlayerCommand> {
        let mut commands = Vec::new();
        for (i, c) in self.compartments.iter().enumerate() {
            let isolated = c.fire > 0.0
                || c.breach_rate > 0.0
                || c.flooding >= 10.0
                || SystemId::ALL
                    .into_iter()
                    .any(|id| id.compartment() as usize == i && self.failed[id as usize]);
            let pumping = battery > 0.0 && (c.flooding > 0.5 || c.breach_rate > 0.0);
            if c.isolated != isolated {
                commands.push(PlayerCommand::SetCompartmentIsolated {
                    compartment: i as u8,
                    isolated,
                });
            }
            if c.pumping != pumping {
                commands.push(PlayerCommand::SetCompartmentPump {
                    compartment: i as u8,
                    enabled: pumping,
                });
            }
        }
        // An unsafe intervention can become invalid after a new impact: reprioritize.
        if let Some(task) = self.intervention {
            let c = self.compartments[task.compartment as usize];
            if task.action != DamageControlAction::Extinguish && c.fire >= 25.0 {
                commands.push(PlayerCommand::CancelDamageControl);
            } else {
                return commands;
            }
        }
        let worst = |predicate: fn(&CompartmentState) -> bool| {
            self.compartments
                .iter()
                .enumerate()
                .filter(|(_, c)| predicate(c))
                .max_by(|(_, a), (_, b)| {
                    (a.fire * 2.0 + a.flooding + a.breach_rate * 100.0)
                        .total_cmp(&(b.fire * 2.0 + b.flooding + b.breach_rate * 100.0))
                })
                .map(|(i, _)| i as u8)
        };
        let next = if let Some(i) = worst(|c| c.fire > 0.0) {
            Some((i, DamageControlAction::Extinguish))
        } else if let Some(i) = worst(|c| c.breach_rate > 0.0 && c.flooding < 90.0) {
            Some((i, DamageControlAction::SealBreach))
        } else {
            SystemId::ALL
                .into_iter()
                .find(|id| {
                    self.failed[*id as usize]
                        && self.compartments[id.compartment() as usize].flooding < 50.0
                })
                .map(|id| (id.compartment(), DamageControlAction::RepairSystem(id)))
        };
        if let Some((compartment, action)) = next {
            commands.push(PlayerCommand::StartDamageControl {
                compartment,
                action,
            });
        }
        commands
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
        if amount >= 8.0 {
            if c.flooding < 60.0 {
                c.fire = (c.fire + amount * 0.3).min(100.0);
            }
            for id in SystemId::ALL {
                if id.compartment() as usize == compartment {
                    self.failed[id as usize] = true;
                }
            }
        }
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
        if !amount.is_finite() || amount <= 0.0 {
            return vec![];
        }
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
        let previous = self.compartments;
        let mut transfer = [0.0; 3];
        for i in 0..2 {
            let (a, b) = (previous[i], previous[i + 1]);
            if !a.isolated && !b.isolated {
                let flow = if a.flooding.max(b.flooding) > 50.0 {
                    ((a.flooding - b.flooding) * config.flooding_transfer * dt).clamp(
                        -(b.flooding - a.flooding).max(0.0) / 2.0,
                        (a.flooding - b.flooding).max(0.0) / 2.0,
                    )
                } else {
                    0.0
                };
                transfer[i] -= flow;
                transfer[i + 1] += flow;
                if a.fire >= 60.0 {
                    self.compartments[i + 1].fire += config.fire_spread * dt;
                }
                if b.fire >= 60.0 {
                    self.compartments[i].fire += config.fire_spread * dt;
                }
            }
        }
        for (i, compartment) in self.compartments.iter_mut().enumerate() {
            let pressure = 1.0 + depth.max(0.0) / crush_depth.max(1.0);
            let pump = if compartment.pumping && self.pumps_powered {
                config.pump_rate
            } else {
                0.0
            };
            compartment.flooding = (compartment.flooding
                + (compartment.breach_rate * pressure - pump) * dt
                + transfer[i])
                .clamp(0.0, 100.0);
            if compartment.fire > 0.0 {
                compartment.fire = (compartment.fire + config.fire_growth * dt).min(100.0);
            }
            if compartment.flooding >= 60.0 {
                compartment.fire = 0.0;
            }
        }
        if self.flooding() >= 100.0 {
            return self.apply(hull, *hull, DamageCause::Flooding, None, tick);
        }
        let fire = self.compartments.iter().map(|c| c.fire).sum::<f32>() / 100.0;
        if fire > 0.0 {
            self.fire_elapsed += dt;
            if self.fire_elapsed >= 1.0 {
                let seconds = self.fire_elapsed.floor();
                self.fire_elapsed -= seconds;
                let events = self.apply(
                    hull,
                    fire * config.fire_damage_per_second * seconds,
                    DamageCause::Fire,
                    None,
                    tick,
                );
                if self.destroyed {
                    return events;
                }
                // Pressure may also act during the same tick.
                let mut all = events;
                all.extend(self.pressure(hull, dt, depth, crush_depth, tick, config));
                return all;
            }
        } else {
            self.fire_elapsed = 0.0;
        }
        self.pressure(hull, dt, depth, crush_depth, tick, config)
    }

    fn pressure(
        &mut self,
        hull: &mut f32,
        dt: f32,
        depth: f32,
        crush_depth: f32,
        tick: u64,
        config: DamageConfig,
    ) -> Vec<GameEvent> {
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
    fn water_transfer_is_conservative_bounded_and_does_not_jump_a_compartment() {
        let mut d = Damage::default();
        let mut hull = 100.0;
        d.compartments[0].flooding = 80.0;
        d.advance(&mut hull, 1.0, 100.0, 250.0, 1, DamageConfig::default());
        assert_eq!(d.compartments[2].flooding, 0.0);
        let total = d.compartments.iter().map(|c| c.flooding).sum::<f32>();
        assert!((total - 80.0).abs() < 0.001);
        for tick in 2..100 {
            d.advance(&mut hull, 0.05, 100.0, 250.0, tick, DamageConfig::default());
        }
        assert!(d
            .compartments
            .iter()
            .all(|c| c.flooding.is_finite() && (0.0..=100.0).contains(&c.flooding)));
        assert!((d.compartments.iter().map(|c| c.flooding).sum::<f32>() - 80.0).abs() < 0.001);
    }

    #[test]
    fn intervention_is_timed_exclusive_cancellable_and_does_not_restore_hull() {
        let mut d = Damage::default();
        let mut hull = 100.0;
        let config = DamageConfig::default();
        d.hit(&mut hull, 20.0, 0, 1, config);
        d.command(
            PlayerCommand::StartDamageControl {
                compartment: 0,
                action: DamageControlAction::SealBreach,
            },
            100.0,
            config,
        )
        .unwrap();
        assert!(d.compartments[0].breach_rate > 0.0);
        assert_eq!(
            d.command(
                PlayerCommand::RepairSystem(SystemId::Torpedo),
                100.0,
                config
            ),
            Err(ProtocolError::DamageControlBusy)
        );
        d.advance_control(10.0, true);
        assert!(d.compartments[0].breach_rate > 0.0);
        d.command(PlayerCommand::CancelDamageControl, 100.0, config)
            .unwrap();
        assert!(d.intervention.is_none());
        d.command(
            PlayerCommand::StartDamageControl {
                compartment: 0,
                action: DamageControlAction::SealBreach,
            },
            100.0,
            config,
        )
        .unwrap();
        assert_eq!(d.intervention.unwrap().remaining, config.seal_seconds);
        d.advance_control(config.seal_seconds, true);
        assert_eq!(d.compartments[0].breach_rate, 0.0);
        assert_eq!(hull, 80.0);
    }

    #[test]
    fn pump_without_sealing_does_not_repair_the_leak_and_needs_power() {
        let mut d = Damage::default();
        let mut hull = 100.0;
        let config = DamageConfig::default();
        d.compartments[0].breach_rate = 0.1;
        d.compartments[0].flooding = 20.0;
        assert_eq!(
            d.command(
                PlayerCommand::SetCompartmentPump {
                    compartment: 0,
                    enabled: true
                },
                0.0,
                config
            ),
            Err(ProtocolError::InsufficientBattery)
        );
        d.command(
            PlayerCommand::SetCompartmentPump {
                compartment: 0,
                enabled: true,
            },
            100.0,
            config,
        )
        .unwrap();
        d.advance_control(1.0, true);
        d.advance(&mut hull, 10.0, 100.0, 250.0, 1, config);
        assert!(d.compartments[0].flooding < 20.0);
        assert_eq!(d.compartments[0].breach_rate, 0.1);
        let water = d.compartments[0].flooding;
        d.advance_control(1.0, false);
        d.advance(&mut hull, 10.0, 100.0, 250.0, 2, config);
        assert!(d.compartments[0].flooding > water);
    }

    #[test]
    fn isolation_stops_adjacent_fire_and_water_transfer_but_disables_equipment() {
        let config = DamageConfig::default();
        let mut open = Damage::default();
        open.compartments[0].fire = 80.0;
        open.compartments[0].flooding = 55.0;
        let mut isolated = Damage::default();
        isolated.compartments = open.compartments;
        isolated
            .command(
                PlayerCommand::SetCompartmentIsolated {
                    compartment: 0,
                    isolated: true,
                },
                100.0,
                config,
            )
            .unwrap();
        assert!(!isolated.available(SystemId::Torpedo));
        let (mut a, mut b) = (100.0, 100.0);
        open.advance(&mut a, 1.0, 100.0, 250.0, 1, config);
        isolated.advance(&mut b, 1.0, 100.0, 250.0, 1, config);
        assert!(open.compartments[1].fire > 0.0);
        assert!(open.compartments[1].flooding > 0.0);
        assert_eq!(open.compartments[2].fire, 0.0);
        assert_eq!(isolated.compartments[1].fire, 0.0);
        assert_eq!(isolated.compartments[1].flooding, 0.0);
    }

    #[test]
    fn malformed_and_unsafe_work_is_rejected_or_paused_without_losing_progress() {
        let mut d = Damage::default();
        let config = DamageConfig::default();
        assert_eq!(
            d.command(
                PlayerCommand::SetCompartmentPump {
                    compartment: 255,
                    enabled: true
                },
                100.0,
                config
            ),
            Err(ProtocolError::InvalidCompartment)
        );
        assert_eq!(
            d.command(PlayerCommand::RepairSystem(SystemId::Engine), 100.0, config),
            Err(ProtocolError::NoDamageToRepair)
        );
        d.compartments[0].breach_rate = 0.1;
        d.compartments[0].fire = 30.0;
        let seal = PlayerCommand::StartDamageControl {
            compartment: 0,
            action: DamageControlAction::SealBreach,
        };
        assert_eq!(
            d.command(seal.clone(), 100.0, config),
            Err(ProtocolError::UnsafeCompartment)
        );
        d.compartments[0].fire = 0.0;
        d.command(seal, 100.0, config).unwrap();
        d.compartments[0].flooding = 95.0;
        d.advance_control(10.0, true);
        assert!(d.intervention.unwrap().paused);
        assert_eq!(d.intervention.unwrap().remaining, config.seal_seconds);
        d.compartments[0].flooding = 20.0;
        d.advance_control(10.0, true);
        assert!(!d.intervention.unwrap().paused);
        assert_eq!(
            d.intervention.unwrap().remaining,
            config.seal_seconds - 10.0
        );
    }

    #[test]
    fn fire_can_destroy_and_its_cause_is_retained_and_cannot_be_repaired_after_loss() {
        let mut d = Damage::default();
        let mut hull = 1.0;
        d.compartments[0].fire = 100.0;
        let events = d.advance(&mut hull, 10.0, 100.0, 250.0, 1, DamageConfig::default());
        assert!(events.contains(&GameEvent::SubmarineDestroyed {
            cause: DamageCause::Fire
        }));
        assert_eq!(d.history.back().unwrap().cause, DamageCause::Fire);
        assert_eq!(
            d.command(
                PlayerCommand::StartDamageControl {
                    compartment: 0,
                    action: DamageControlAction::Extinguish
                },
                100.0,
                DamageConfig::default()
            ),
            Err(ProtocolError::SubmarineDestroyed)
        );
    }

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
