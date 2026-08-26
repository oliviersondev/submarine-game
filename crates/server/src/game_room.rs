use std::collections::HashMap;

use shared::{CrewRole, GameEvent, PlayerCommand, ProtocolError, ServerMessage};
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
                    Some((role, cmd)) => handle_command(&mut sim, &players, role, cmd),
                    None => break,
                }
            }
        }
    }
}

fn handle_command(
    sim: &mut Simulation,
    players: &HashMap<u32, (CrewRole, mpsc::Sender<ServerMessage>)>,
    role: CrewRole,
    command: PlayerCommand,
) {
    if !command_allowed_for_role(role, &command) {
        send_to_role(
            players,
            role,
            ServerMessage::Error(ProtocolError::CommandNotAllowedForRole),
        );
        return;
    }

    for event in sim.apply_command(command) {
        broadcast(players, &ServerMessage::Event(event));
    }
}

fn command_allowed_for_role(role: CrewRole, command: &PlayerCommand) -> bool {
    matches!(
        (role, command),
        (
            CrewRole::Pilot,
            PlayerCommand::SetHeading(_) | PlayerCommand::SetDepth(_) | PlayerCommand::SetSpeed(_)
        ) | (CrewRole::Sonar, PlayerCommand::SonarPing)
            | (CrewRole::Engineer, PlayerCommand::RepairSystem(_))
            | (CrewRole::Weapons, PlayerCommand::FireTorpedo { .. })
    )
}

fn send_to_role(
    players: &HashMap<u32, (CrewRole, mpsc::Sender<ServerMessage>)>,
    role: CrewRole,
    message: ServerMessage,
) {
    if let Some((_, tx)) = players
        .values()
        .find(|(player_role, _)| *player_role == role)
    {
        let _ = tx.try_send(message);
    }
}

fn broadcast(players: &HashMap<u32, (CrewRole, mpsc::Sender<ServerMessage>)>, msg: &ServerMessage) {
    for (_, tx) in players.values() {
        let _ = tx.try_send(msg.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_commands_for_their_assigned_roles() {
        assert!(command_allowed_for_role(
            CrewRole::Pilot,
            &PlayerCommand::SetHeading(90.0)
        ));
        assert!(command_allowed_for_role(
            CrewRole::Pilot,
            &PlayerCommand::SetDepth(50.0)
        ));
        assert!(command_allowed_for_role(
            CrewRole::Pilot,
            &PlayerCommand::SetSpeed(10.0)
        ));
        assert!(command_allowed_for_role(
            CrewRole::Sonar,
            &PlayerCommand::SonarPing
        ));
        assert!(command_allowed_for_role(
            CrewRole::Engineer,
            &PlayerCommand::RepairSystem(shared::SystemId::Engine)
        ));
        assert!(command_allowed_for_role(
            CrewRole::Weapons,
            &PlayerCommand::FireTorpedo { bearing: 45.0 }
        ));
    }

    #[test]
    fn rejects_commands_from_other_roles() {
        assert!(!command_allowed_for_role(
            CrewRole::Captain,
            &PlayerCommand::SetHeading(90.0)
        ));
        assert!(!command_allowed_for_role(
            CrewRole::Pilot,
            &PlayerCommand::FireTorpedo { bearing: 45.0 }
        ));
        assert!(!command_allowed_for_role(
            CrewRole::Weapons,
            &PlayerCommand::RepairSystem(shared::SystemId::Engine)
        ));
    }

    #[test]
    fn rejected_command_does_not_change_state_and_notifies_sender() {
        let (tx, mut rx) = mpsc::channel(1);
        let players = HashMap::from([(1, (CrewRole::Captain, tx))]);
        let mut sim = Simulation::new();

        handle_command(
            &mut sim,
            &players,
            CrewRole::Captain,
            PlayerCommand::SetDepth(50.0),
        );

        assert_eq!(sim.state.depth, 0.0);
        assert!(matches!(
            rx.try_recv(),
            Ok(ServerMessage::Error(
                ProtocolError::CommandNotAllowedForRole
            ))
        ));
    }
}
