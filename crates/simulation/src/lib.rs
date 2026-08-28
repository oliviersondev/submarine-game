use shared::{
    AcousticLevel, AlertKind, BallastState, DiveState, GameEvent, MissionConfig, PilotOrder,
    PlayerCommand, SubmarineState, ALERT_KINDS,
};

const KNOTS_TO_METERS_PER_SECOND: f32 = 0.514_444;

pub struct Simulation {
    pub state: SubmarineState,
    pub config: MissionConfig,
    pub tick: u64,
    active_alerts: [bool; ALERT_KINDS.len()],
}

impl Simulation {
    pub fn new() -> Self {
        Self::with_config(MissionConfig::new(0))
    }

    pub fn with_config(config: MissionConfig) -> Self {
        Self {
            state: SubmarineState::default(),
            config,
            tick: 0,
            active_alerts: [false; ALERT_KINDS.len()],
        }
    }

    pub fn tick(&mut self, dt: f32) -> Vec<GameEvent> {
        if !dt.is_finite() || dt <= 0.0 {
            return vec![];
        }

        self.update_dive_state();
        self.enforce_air_intake_constraints();
        self.update_heading(dt);
        self.update_depth(dt);
        self.update_dive_state();
        self.enforce_air_intake_constraints();
        self.update_speed(dt);
        self.update_position(dt);
        self.update_resources(dt);
        self.update_acoustic_signature();
        self.sanitize_state();
        self.tick = self.tick.wrapping_add(1);
        self.update_alerts()
    }

    pub fn apply_command(&mut self, command: PlayerCommand) -> Vec<GameEvent> {
        let parameters = self.config.submarine;
        match command {
            PlayerCommand::SetHeading(value) if value.is_finite() => {
                self.state.ordered_heading = value.rem_euclid(360.0);
            }
            PlayerCommand::SetDepth(value) if value.is_finite() => {
                self.state.ordered_depth = value.clamp(0.0, parameters.crush_depth);
                if self.state.ordered_depth > 0.5 && self.state.emergency_surface {
                    self.state.emergency_surface = false;
                    self.state.ballast = BallastState::Hold;
                }
            }
            PlayerCommand::SetSpeed(value) if value.is_finite() => {
                self.state.ordered_speed = value.clamp(0.0, parameters.max_surface_speed);
            }
            PlayerCommand::SetBallast(ballast) => {
                self.state.ballast = ballast;
            }
            PlayerCommand::EmergencySurface => {
                self.state.emergency_surface = true;
                self.state.ordered_depth = 0.0;
                self.state.ballast = BallastState::Blow;
                return vec![GameEvent::EmergencySurfaceStarted];
            }
            PlayerCommand::SetDiesels(enabled) => {
                self.state.propulsion.diesels_on = enabled && self.air_intake_available();
            }
            PlayerCommand::SetElectricMotors(enabled) => {
                self.state.propulsion.electric_motors_on = enabled;
            }
            PlayerCommand::SetVentilation(enabled) => {
                self.state.propulsion.ventilation_on = enabled && self.air_intake_available();
            }
            PlayerCommand::SetBatteryCharging(enabled) => {
                self.state.propulsion.charging = enabled;
            }
            PlayerCommand::FireTorpedo { bearing } if bearing.is_finite() => {
                return vec![GameEvent::TorpedoFired {
                    bearing: bearing.rem_euclid(360.0),
                }];
            }
            PlayerCommand::RepairSystem(id) => {
                return vec![GameEvent::SystemRepaired(id)];
            }
            PlayerCommand::SonarPing => {}
            _ => {}
        }
        vec![]
    }

    pub fn apply_pilot_order(&mut self, order: PilotOrder) -> Vec<GameEvent> {
        let mut events = Vec::new();
        for command in [
            PlayerCommand::SetHeading(order.heading),
            PlayerCommand::SetSpeed(order.speed),
            PlayerCommand::SetDepth(order.depth),
        ] {
            events.extend(self.apply_command(command));
        }
        self.state.ballast = if order.depth > self.state.depth + 1.0 {
            BallastState::Flood
        } else if order.depth + 1.0 < self.state.depth {
            BallastState::Blow
        } else {
            BallastState::Hold
        };
        events
    }

    pub fn automate_engineer(&mut self) {
        let intake = self.air_intake_available();
        self.state.propulsion.electric_motors_on = true;
        self.state.propulsion.diesels_on = intake;
        self.state.propulsion.ventilation_on = intake;
        self.state.propulsion.charging = intake && self.state.battery < 99.9;
    }

    pub fn air_intake_available(&self) -> bool {
        self.state.depth <= 0.5 && self.state.dive_state == DiveState::Surface
    }

    pub fn active_alerts(&self) -> Vec<AlertKind> {
        ALERT_KINDS
            .into_iter()
            .zip(self.active_alerts)
            .filter_map(|(alert, active)| active.then_some(alert))
            .collect()
    }

    fn update_heading(&mut self, dt: f32) {
        let delta = shortest_angle(self.state.heading, self.state.ordered_heading);
        let turn = delta.clamp(
            -self.config.submarine.turn_rate * dt,
            self.config.submarine.turn_rate * dt,
        );
        self.state.heading = (self.state.heading + turn).rem_euclid(360.0);
        self.state.turn_rate = turn / dt;
    }

    fn update_depth(&mut self, dt: f32) {
        let parameters = self.config.submarine;
        let target_rate = if self.state.emergency_surface {
            -parameters.emergency_ascent_speed
        } else {
            let error = self.state.ordered_depth - self.state.depth;
            let ballast_factor = match self.state.ballast {
                BallastState::Flood => 1.0,
                BallastState::Blow => -1.0,
                BallastState::Hold => error.clamp(-1.0, 1.0),
            };
            parameters.vertical_speed * ballast_factor
        };

        self.state.vertical_speed = target_rate;
        let previous_depth = self.state.depth;
        self.state.depth = (self.state.depth + target_rate * dt).clamp(0.0, parameters.crush_depth);

        if (self.state.ordered_depth - previous_depth).signum()
            != (self.state.ordered_depth - self.state.depth).signum()
            && !self.state.emergency_surface
        {
            self.state.depth = self.state.ordered_depth;
            self.state.vertical_speed = 0.0;
            self.state.ballast = BallastState::Hold;
        }
        if self.state.depth <= 0.0 {
            self.state.depth = 0.0;
            self.state.vertical_speed = 0.0;
            self.state.emergency_surface = false;
            self.state.ballast = BallastState::Hold;
        }
    }

    fn update_dive_state(&mut self) {
        self.state.dive_state = if self.state.depth <= 0.5 {
            DiveState::Surface
        } else if self.state.depth <= self.config.submarine.periscope_depth + 0.5 {
            DiveState::Periscope
        } else {
            DiveState::Submerged
        };
    }

    fn enforce_air_intake_constraints(&mut self) {
        if !self.air_intake_available() {
            self.state.propulsion.diesels_on = false;
            self.state.propulsion.ventilation_on = false;
        }
    }

    fn update_speed(&mut self, dt: f32) {
        let parameters = self.config.submarine;
        let max_speed = if self.state.dive_state == DiveState::Surface {
            parameters.max_surface_speed
        } else {
            parameters.max_submerged_speed
        };
        let propulsion_available = (self.air_intake_available()
            && self.state.propulsion.diesels_on)
            || (self.state.propulsion.electric_motors_on && self.state.battery > 0.0);
        let target = if propulsion_available {
            self.state.ordered_speed.min(max_speed)
        } else {
            0.0
        };
        let rate = if target >= self.state.speed {
            parameters.acceleration
        } else {
            parameters.deceleration
        };
        self.state.speed = move_towards(self.state.speed, target, rate * dt);
    }

    fn update_position(&mut self, dt: f32) {
        let distance = self.state.speed * KNOTS_TO_METERS_PER_SECOND * dt;
        let heading = self.state.heading.to_radians();
        self.state.x += heading.sin() * distance;
        self.state.y += heading.cos() * distance;
    }

    fn update_resources(&mut self, dt: f32) {
        let parameters = self.config.submarine;
        let submerged = !self.air_intake_available();
        let speed_ratio = if submerged {
            self.state.speed / parameters.max_submerged_speed
        } else {
            self.state.speed / parameters.max_surface_speed
        }
        .clamp(0.0, 1.0);
        let propulsion_load = if self.state.propulsion.electric_motors_on
            && (submerged || !self.state.propulsion.diesels_on)
        {
            parameters.battery_drain_at_full_speed * speed_ratio.powi(3)
        } else {
            0.0
        };
        let hotel_load = parameters.battery_drain_base;
        self.state.electrical_load = hotel_load + propulsion_load;

        let recharge = if self.air_intake_available()
            && self.state.propulsion.diesels_on
            && self.state.propulsion.charging
        {
            parameters.battery_recharge_rate
        } else {
            0.0
        };
        self.state.battery =
            (self.state.battery + (recharge - self.state.electrical_load) * dt).clamp(0.0, 100.0);

        if self.air_intake_available() && self.state.propulsion.ventilation_on {
            self.state.oxygen =
                (self.state.oxygen + parameters.oxygen_ventilation_rate * dt).clamp(0.0, 100.0);
        } else {
            self.state.oxygen =
                (self.state.oxygen - parameters.oxygen_drain_rate * dt).clamp(0.0, 100.0);
        }
    }

    fn update_acoustic_signature(&mut self) {
        let parameters = self.config.submarine;
        let max_speed = if self.state.dive_state == DiveState::Surface {
            parameters.max_surface_speed
        } else {
            parameters.max_submerged_speed
        };
        let propeller = (self.state.speed / max_speed).clamp(0.0, 1.0) * 45.0;
        let diesels = if self.state.propulsion.diesels_on {
            25.0
        } else {
            0.0
        };
        let motors = if self.state.propulsion.electric_motors_on && self.state.speed > 0.1 {
            8.0
        } else {
            0.0
        };
        let ventilation = if self.state.propulsion.ventilation_on {
            7.0
        } else {
            0.0
        };
        let cavitation_speed = if self.state.depth < parameters.periscope_depth {
            max_speed * 0.65
        } else {
            max_speed * 0.85
        };
        self.state.cavitating = self.state.speed > cavitation_speed;
        let cavitation = if self.state.cavitating { 35.0 } else { 0.0 };
        self.state.acoustic_signature =
            (propeller + diesels + motors + ventilation + cavitation).clamp(0.0, 100.0);
        self.state.acoustic_level = match self.state.acoustic_signature {
            value if value < 10.0 => AcousticLevel::Silent,
            value if value < 25.0 => AcousticLevel::Low,
            value if value < 50.0 => AcousticLevel::Notable,
            value if value < 75.0 => AcousticLevel::Loud,
            _ => AcousticLevel::Critical,
        };
    }

    fn update_alerts(&mut self) -> Vec<GameEvent> {
        let parameters = self.config.submarine;
        let current = [
            self.state.battery <= parameters.low_battery_threshold,
            self.state.oxygen <= parameters.critical_air_threshold,
            self.state.cavitating,
            self.state.depth >= parameters.critical_depth,
        ];
        let mut events = Vec::new();
        for (index, alert) in ALERT_KINDS.into_iter().enumerate() {
            match (self.active_alerts[index], current[index]) {
                (false, true) => events.push(GameEvent::AlertRaised(alert)),
                (true, false) => events.push(GameEvent::AlertCleared(alert)),
                _ => {}
            }
        }
        self.active_alerts = current;
        events
    }

    fn sanitize_state(&mut self) {
        let parameters = self.config.submarine;
        for value in [
            &mut self.state.x,
            &mut self.state.y,
            &mut self.state.depth,
            &mut self.state.heading,
            &mut self.state.speed,
            &mut self.state.turn_rate,
            &mut self.state.vertical_speed,
            &mut self.state.battery,
            &mut self.state.oxygen,
            &mut self.state.electrical_load,
            &mut self.state.acoustic_signature,
        ] {
            if !value.is_finite() {
                *value = 0.0;
            }
        }
        self.state.depth = self.state.depth.clamp(0.0, parameters.crush_depth);
        self.state.speed = self.state.speed.clamp(0.0, parameters.max_surface_speed);
        self.state.battery = self.state.battery.clamp(0.0, 100.0);
        self.state.oxygen = self.state.oxygen.clamp(0.0, 100.0);
    }
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

fn move_towards(current: f32, target: f32, maximum_delta: f32) -> f32 {
    if (target - current).abs() <= maximum_delta {
        target
    } else {
        current + (target - current).signum() * maximum_delta
    }
}

fn shortest_angle(current: f32, target: f32) -> f32 {
    (target - current + 180.0).rem_euclid(360.0) - 180.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_approx_eq(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
    }

    #[test]
    fn commands_set_orders_without_teleporting_state() {
        let mut sim = Simulation::new();
        sim.apply_command(PlayerCommand::SetHeading(90.0));
        sim.apply_command(PlayerCommand::SetSpeed(10.0));
        sim.apply_command(PlayerCommand::SetDepth(50.0));

        assert_eq!(sim.state.ordered_heading, 90.0);
        assert_eq!(sim.state.ordered_speed, 10.0);
        assert_eq!(sim.state.ordered_depth, 50.0);
        assert_eq!(sim.state.heading, 0.0);
        assert_eq!(sim.state.speed, 0.0);
        assert_eq!(sim.state.depth, 0.0);
    }

    #[test]
    fn movement_rates_are_bounded() {
        let mut sim = Simulation::new();
        sim.apply_pilot_order(PilotOrder {
            heading: 90.0,
            speed: 18.0,
            depth: 100.0,
        });
        sim.tick(1.0);

        assert_approx_eq(sim.state.heading, sim.config.submarine.turn_rate);
        assert_approx_eq(sim.state.speed, sim.config.submarine.acceleration);
        assert_approx_eq(sim.state.depth, sim.config.submarine.vertical_speed);
    }

    #[test]
    fn non_finite_commands_are_ignored_and_state_stays_finite() {
        let mut sim = Simulation::new();
        sim.apply_command(PlayerCommand::SetHeading(f32::NAN));
        sim.apply_command(PlayerCommand::SetDepth(f32::INFINITY));
        sim.apply_command(PlayerCommand::SetSpeed(f32::NEG_INFINITY));
        sim.tick(0.05);

        assert!(sim.state.x.is_finite());
        assert!(sim.state.y.is_finite());
        assert!(sim.state.depth.is_finite());
        assert!(sim.state.heading.is_finite());
        assert!(sim.state.speed.is_finite());
    }

    #[test]
    fn electrical_consumption_increases_with_speed() {
        let mut slow = submerged_simulation(1.0);
        let mut fast = submerged_simulation(8.0);
        slow.tick(1.0);
        fast.tick(1.0);

        assert!(fast.state.electrical_load > slow.state.electrical_load);
    }

    #[test]
    fn diesels_cannot_run_without_air_intake() {
        let mut sim = submerged_simulation(0.0);
        sim.apply_command(PlayerCommand::SetDiesels(true));
        sim.tick(0.05);
        assert!(!sim.state.propulsion.diesels_on);
    }

    #[test]
    fn battery_recharges_at_surface_and_discharges_submerged() {
        let mut surface = Simulation::new();
        surface.state.battery = 50.0;
        surface.tick(10.0);
        assert!(surface.state.battery > 50.0);

        let mut submerged = submerged_simulation(6.0);
        let initial = submerged.state.battery;
        submerged.tick(10.0);
        assert!(submerged.state.battery < initial);
    }

    #[test]
    fn dive_states_transition_reproducibly() {
        let mut first = Simulation::new();
        let mut second = Simulation::new();
        for sim in [&mut first, &mut second] {
            sim.apply_pilot_order(PilotOrder {
                heading: 0.0,
                speed: 2.0,
                depth: 40.0,
            });
            for _ in 0..600 {
                sim.tick(0.05);
            }
        }
        assert_eq!(first.state, second.state);
        assert_eq!(first.state.dive_state, DiveState::Submerged);
    }

    #[test]
    fn threshold_events_are_edge_triggered() {
        let mut sim = submerged_simulation(0.0);
        sim.state.battery = sim.config.submarine.low_battery_threshold;
        assert_eq!(
            sim.tick(0.05),
            vec![GameEvent::AlertRaised(AlertKind::BatteryLow)]
        );
        assert!(sim.tick(0.05).is_empty());
        sim.state.battery = 50.0;
        assert_eq!(
            sim.tick(0.05),
            vec![GameEvent::AlertCleared(AlertKind::BatteryLow)]
        );
    }

    #[test]
    fn emergency_surface_returns_to_surface() {
        let mut sim = submerged_simulation(2.0);
        sim.apply_command(PlayerCommand::EmergencySurface);
        for _ in 0..400 {
            sim.tick(0.05);
        }
        assert_eq!(sim.state.depth, 0.0);
        assert_eq!(sim.state.dive_state, DiveState::Surface);
        assert!(!sim.state.emergency_surface);
    }

    #[test]
    fn depth_order_cancels_emergency_ascent() {
        let mut sim = submerged_simulation(2.0);
        sim.apply_command(PlayerCommand::EmergencySurface);
        sim.apply_command(PlayerCommand::SetDepth(80.0));
        sim.tick(1.0);

        assert!(!sim.state.emergency_surface);
        assert_eq!(sim.state.ballast, BallastState::Hold);
        assert!(sim.state.depth > 50.0);
    }

    #[test]
    fn engineer_automation_never_issues_pilot_commands() {
        let mut sim = submerged_simulation(2.0);
        sim.state.battery = sim.config.submarine.low_battery_threshold;
        sim.state.oxygen = sim.config.submarine.critical_air_threshold;
        sim.automate_engineer();

        assert!(!sim.state.emergency_surface);
        assert_eq!(sim.state.ordered_depth, 50.0);
    }

    #[test]
    fn same_seed_and_orders_are_deterministic() {
        let config = MissionConfig::new(1234);
        let order = PilotOrder {
            heading: 72.0,
            speed: 7.0,
            depth: 80.0,
        };
        let mut first = Simulation::with_config(config);
        let mut second = Simulation::with_config(config);
        first.apply_pilot_order(order);
        second.apply_pilot_order(order);
        for _ in 0..100 {
            first.tick(0.05);
            second.tick(0.05);
        }
        assert_eq!(first.state, second.state);
        assert_eq!(first.active_alerts, second.active_alerts);
    }

    #[test]
    fn submarine_can_dive_run_silently_and_resurface_with_resources() {
        let mut sim = Simulation::new();
        sim.apply_pilot_order(PilotOrder {
            heading: 45.0,
            speed: 2.0,
            depth: 40.0,
        });
        for _ in 0..600 {
            sim.automate_engineer();
            sim.tick(0.05);
        }

        assert_eq!(sim.state.dive_state, DiveState::Submerged);
        assert_approx_eq(sim.state.depth, 40.0);
        assert!(matches!(
            sim.state.acoustic_level,
            AcousticLevel::Silent | AcousticLevel::Low
        ));

        sim.apply_pilot_order(PilotOrder {
            heading: 45.0,
            speed: 2.0,
            depth: 0.0,
        });
        for _ in 0..600 {
            sim.automate_engineer();
            sim.tick(0.05);
        }

        assert_eq!(sim.state.dive_state, DiveState::Surface);
        assert_eq!(sim.state.depth, 0.0);
        assert!(sim.state.battery > 0.0);
        assert!(sim.state.oxygen > 0.0);
    }

    fn submerged_simulation(speed: f32) -> Simulation {
        let mut sim = Simulation::new();
        sim.state.depth = 50.0;
        sim.state.ordered_depth = 50.0;
        sim.state.dive_state = DiveState::Submerged;
        sim.state.speed = speed;
        sim.state.ordered_speed = speed;
        sim.state.propulsion.diesels_on = false;
        sim.state.propulsion.ventilation_on = false;
        sim
    }
}
