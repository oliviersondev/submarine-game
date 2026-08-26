use simulation::Simulation;
use shared::{CrewRole, GameEvent, PlayerCommand};
use tokio::sync::mpsc;

pub struct GameRoom {
    simulation: Simulation,
}

impl GameRoom {
    pub fn new() -> Self {
        Self {
            simulation: Simulation::new(),
        }
    }

    pub fn apply(&mut self, role: CrewRole, command: PlayerCommand) -> Vec<GameEvent> {
        // TODO: validate command against role before applying
        let _ = role;
        self.simulation.apply_command(command)
    }
}

pub type CommandSender = mpsc::Sender<(CrewRole, PlayerCommand)>;
pub type CommandReceiver = mpsc::Receiver<(CrewRole, PlayerCommand)>;
