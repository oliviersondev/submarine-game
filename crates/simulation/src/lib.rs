use shared::{GameEvent, PlayerCommand, SubmarineState};

const MAX_SPEED_KNOTS: f32 = 20.0;
const MAX_DEPTH_METERS: f32 = 1_000.0;
const KNOTS_TO_METERS_PER_SECOND: f32 = 0.514_444;

pub struct Simulation {
    pub state: SubmarineState,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            state: SubmarineState::default(),
        }
    }

    pub fn tick(&mut self, dt: f32) -> Vec<GameEvent> {
        if !dt.is_finite() || dt <= 0.0 {
            return vec![];
        }

        let distance = self.state.speed * KNOTS_TO_METERS_PER_SECOND * dt;
        let heading = self.state.heading.to_radians();
        self.state.x += heading.sin() * distance;
        self.state.y += heading.cos() * distance;

        vec![]
    }

    pub fn apply_command(&mut self, command: PlayerCommand) -> Vec<GameEvent> {
        match command {
            PlayerCommand::SetHeading(h) if h.is_finite() => {
                self.state.heading = h.rem_euclid(360.0)
            }
            PlayerCommand::SetDepth(d) if d.is_finite() => {
                self.state.depth = d.clamp(0.0, MAX_DEPTH_METERS)
            }
            PlayerCommand::SetSpeed(s) if s.is_finite() => {
                self.state.speed = s.clamp(0.0, MAX_SPEED_KNOTS)
            }
            PlayerCommand::FireTorpedo { bearing } => {
                if bearing.is_finite() {
                    return vec![GameEvent::TorpedoFired {
                        bearing: bearing.rem_euclid(360.0),
                    }];
                }
            }
            PlayerCommand::RepairSystem(id) => {
                return vec![GameEvent::SystemRepaired(id)];
            }
            PlayerCommand::SonarPing => {}
            _ => {}
        }
        vec![]
    }
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::PlayerCommand;

    fn assert_approx_eq(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.000_1,
            "{actual} != {expected}"
        );
    }

    #[test]
    fn heading_updates() {
        let mut sim = Simulation::new();
        sim.apply_command(PlayerCommand::SetHeading(90.0));
        assert_eq!(sim.state.heading, 90.0);
    }

    #[test]
    fn depth_updates() {
        let mut sim = Simulation::new();
        sim.apply_command(PlayerCommand::SetDepth(50.0));
        assert_eq!(sim.state.depth, 50.0);
    }

    #[test]
    fn pilot_commands_are_normalized_and_clamped() {
        let mut sim = Simulation::new();

        sim.apply_command(PlayerCommand::SetHeading(-90.0));
        sim.apply_command(PlayerCommand::SetDepth(2_000.0));
        sim.apply_command(PlayerCommand::SetSpeed(30.0));

        assert_eq!(sim.state.heading, 270.0);
        assert_eq!(sim.state.depth, MAX_DEPTH_METERS);
        assert_eq!(sim.state.speed, MAX_SPEED_KNOTS);
    }

    #[test]
    fn non_finite_pilot_commands_are_ignored() {
        let mut sim = Simulation::new();
        sim.state.heading = 45.0;
        sim.state.depth = 100.0;
        sim.state.speed = 5.0;

        sim.apply_command(PlayerCommand::SetHeading(f32::NAN));
        sim.apply_command(PlayerCommand::SetDepth(f32::INFINITY));
        sim.apply_command(PlayerCommand::SetSpeed(f32::NEG_INFINITY));

        assert_eq!(sim.state.heading, 45.0);
        assert_eq!(sim.state.depth, 100.0);
        assert_eq!(sim.state.speed, 5.0);
    }

    #[test]
    fn tick_moves_using_nautical_heading_and_knots() {
        let mut northbound = Simulation::new();
        northbound.state.speed = 10.0;
        northbound.tick(1.0);
        assert_approx_eq(northbound.state.x, 0.0);
        assert_approx_eq(northbound.state.y, 5.144_44);

        let mut eastbound = Simulation::new();
        eastbound.state.heading = 90.0;
        eastbound.state.speed = 10.0;
        eastbound.tick(1.0);
        assert_approx_eq(eastbound.state.x, 5.144_44);
        assert_approx_eq(eastbound.state.y, 0.0);
    }

    #[test]
    fn tick_ignores_invalid_or_non_positive_delta_time() {
        let mut sim = Simulation::new();
        sim.state.speed = 10.0;

        sim.tick(0.0);
        sim.tick(-1.0);
        sim.tick(f32::NAN);

        assert_eq!(sim.state.x, 0.0);
        assert_eq!(sim.state.y, 0.0);
    }

    #[test]
    fn tick_returns_no_events_by_default() {
        let mut sim = Simulation::new();
        let events = sim.tick(0.1);
        assert!(events.is_empty());
    }
}
