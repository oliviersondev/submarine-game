use shared::{GameEvent, PlayerCommand, SubmarineState};

pub struct Simulation {
    pub state: SubmarineState,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            state: SubmarineState::default(),
        }
    }

    pub fn tick(&mut self, _dt: f32) -> Vec<GameEvent> {
        vec![]
    }

    pub fn apply_command(&mut self, command: PlayerCommand) -> Vec<GameEvent> {
        match command {
            PlayerCommand::SetHeading(h) => self.state.heading = h,
            PlayerCommand::SetDepth(d) => self.state.depth = d,
            PlayerCommand::SetSpeed(s) => self.state.speed = s,
            PlayerCommand::FireTorpedo { bearing } => {
                return vec![GameEvent::TorpedoFired { bearing }];
            }
            PlayerCommand::RepairSystem(id) => {
                return vec![GameEvent::SystemRepaired(id)];
            }
            PlayerCommand::SonarPing => {}
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

    #[test]
    fn heading_updates() {
        let mut sim = Simulation::new();
        sim.apply_command(PlayerCommand::SetHeading(90.0));
        assert_eq!(sim.state.heading, 90.0);
    }

    #[test]
    fn tick_returns_no_events_by_default() {
        let mut sim = Simulation::new();
        let events = sim.tick(0.1);
        assert!(events.is_empty());
    }
}
