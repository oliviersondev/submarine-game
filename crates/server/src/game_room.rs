use std::collections::HashMap;

use shared::{CrewRole, GameEvent, PlayerCommand, ServerMessage};
use simulation::Simulation;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

pub async fn run(
    mut cmd_rx: mpsc::Receiver<(CrewRole, PlayerCommand)>,
    players: HashMap<u32, (CrewRole, mpsc::Sender<ServerMessage>)>,
) {
    let mut sim = Simulation::new();
    let mut ticker = interval(Duration::from_millis(50)); // 20 Hz

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let events = sim.tick(0.05);
                broadcast(&players, &ServerMessage::Event(GameEvent::StateSnapshot(sim.state.clone())));
                for event in events {
                    broadcast(&players, &ServerMessage::Event(event));
                }
            }
            result = cmd_rx.recv() => {
                match result {
                    Some((_role, cmd)) => {
                        // TODO: validate cmd against _role
                        let events = sim.apply_command(cmd);
                        for event in events {
                            broadcast(&players, &ServerMessage::Event(event));
                        }
                    }
                    None => break,
                }
            }
        }
    }
}

fn broadcast(players: &HashMap<u32, (CrewRole, mpsc::Sender<ServerMessage>)>, msg: &ServerMessage) {
    for (_, tx) in players.values() {
        let _ = tx.try_send(msg.clone());
    }
}
