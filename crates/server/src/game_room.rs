use std::collections::{HashMap, HashSet};

use shared::{
    CommandId, CrewRole, MissionConfig, PilotOrder, PlayerCommand, PlayerId, ProtocolError,
    ServerMessage, ServerPayload,
};
use simulation::Simulation;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

pub enum GameRoomAction {
    Player(PlayerCommand),
    OrderPilotBot(PilotOrder),
}

pub struct GameRoomCommand {
    pub player_id: PlayerId,
    pub role: CrewRole,
    pub command_id: CommandId,
    pub action: GameRoomAction,
}

pub async fn run(
    config: MissionConfig,
    mut cmd_rx: mpsc::Receiver<GameRoomCommand>,
    players: HashMap<PlayerId, (CrewRole, mpsc::Sender<ServerMessage>)>,
) {
    let mut sim = Simulation::with_config(config);
    let human_roles: HashSet<_> = players.values().map(|(role, _)| *role).collect();
    let mut ticker = interval(Duration::from_millis(50));
    let mut snapshot_id = 0_u64;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                for event in sim.tick(0.05) {
                    broadcast(&players, ServerPayload::Event { tick: sim.tick, event });
                }
                snapshot_id = snapshot_id.wrapping_add(1);
                broadcast(&players, ServerPayload::Snapshot {
                    snapshot_id,
                    tick: sim.tick,
                    submarine: sim.state.clone(),
                });
            }
            result = cmd_rx.recv() => {
                match result {
                    Some(command) => handle_command(&mut sim, &players, &human_roles, command),
                    None => break,
                }
            }
        }
    }
}

fn handle_command(
    sim: &mut Simulation,
    players: &HashMap<PlayerId, (CrewRole, mpsc::Sender<ServerMessage>)>,
    human_roles: &HashSet<CrewRole>,
    request: GameRoomCommand,
) {
    let result = match request.action {
        GameRoomAction::Player(command) => {
            if command_allowed_for_role(request.role, &command) {
                Ok(sim.apply_command(command))
            } else {
                Err(ProtocolError::CommandNotAllowedForRole)
            }
        }
        GameRoomAction::OrderPilotBot(order) => {
            if request.role != CrewRole::Captain {
                Err(ProtocolError::CommandNotAllowedForRole)
            } else if human_roles.contains(&CrewRole::Pilot) {
                Err(ProtocolError::PilotControlledByHuman)
            } else {
                Ok(sim.apply_pilot_order(order))
            }
        }
    };

    match result {
        Ok(events) => {
            for event in events {
                broadcast(
                    players,
                    ServerPayload::Event {
                        tick: sim.tick,
                        event,
                    },
                );
            }
        }
        Err(error) => send_to_player(
            players,
            request.player_id,
            ServerPayload::Error {
                command_id: Some(request.command_id),
                error,
            },
        ),
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

fn send_to_player(
    players: &HashMap<PlayerId, (CrewRole, mpsc::Sender<ServerMessage>)>,
    player_id: PlayerId,
    payload: ServerPayload,
) {
    if let Some((_, tx)) = players.get(&player_id) {
        let _ = tx.try_send(ServerMessage::new(payload));
    }
}

fn broadcast(
    players: &HashMap<PlayerId, (CrewRole, mpsc::Sender<ServerMessage>)>,
    payload: ServerPayload,
) {
    let message = ServerMessage::new(payload);
    for (_, tx) in players.values() {
        let _ = tx.try_send(message.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(role: CrewRole, action: GameRoomAction) -> GameRoomCommand {
        GameRoomCommand {
            player_id: PlayerId(1),
            role,
            command_id: CommandId(7),
            action,
        }
    }

    #[test]
    fn allows_commands_for_their_assigned_roles() {
        assert!(command_allowed_for_role(
            CrewRole::Pilot,
            &PlayerCommand::SetHeading(90.0)
        ));
        assert!(command_allowed_for_role(
            CrewRole::Sonar,
            &PlayerCommand::SonarPing
        ));
        assert!(!command_allowed_for_role(
            CrewRole::Captain,
            &PlayerCommand::SetHeading(90.0)
        ));
    }

    #[test]
    fn pilot_bot_applies_only_pilot_controls() {
        let (tx, _rx) = mpsc::channel(2);
        let players = HashMap::from([(PlayerId(1), (CrewRole::Captain, tx))]);
        let humans = HashSet::from([CrewRole::Captain]);
        let mut sim = Simulation::new();

        handle_command(
            &mut sim,
            &players,
            &humans,
            request(
                CrewRole::Captain,
                GameRoomAction::OrderPilotBot(PilotOrder {
                    heading: 80.0,
                    speed: 9.0,
                    depth: 60.0,
                }),
            ),
        );

        assert_eq!(sim.state.heading, 80.0);
        assert_eq!(sim.state.speed, 9.0);
        assert_eq!(sim.state.depth, 60.0);
    }

    #[test]
    fn pilot_bot_order_is_blocked_when_pilot_is_human() {
        let (tx, mut rx) = mpsc::channel(2);
        let players = HashMap::from([(PlayerId(1), (CrewRole::Captain, tx))]);
        let humans = HashSet::from([CrewRole::Captain, CrewRole::Pilot]);
        let mut sim = Simulation::new();

        handle_command(
            &mut sim,
            &players,
            &humans,
            request(
                CrewRole::Captain,
                GameRoomAction::OrderPilotBot(PilotOrder {
                    heading: 80.0,
                    speed: 9.0,
                    depth: 60.0,
                }),
            ),
        );

        assert_eq!(sim.state, shared::SubmarineState::default());
        assert!(matches!(
            rx.try_recv(),
            Ok(ServerMessage {
                payload: ServerPayload::Error {
                    error: ProtocolError::PilotControlledByHuman,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn two_room_simulations_evolve_independently() {
        let config = MissionConfig { seed: 44 };
        let mut first = Simulation::with_config(config);
        let mut second = Simulation::with_config(config);
        first.apply_command(PlayerCommand::SetSpeed(10.0));

        first.tick(1.0);
        second.tick(1.0);

        assert_ne!(first.state, second.state);
        assert_eq!(second.state, shared::SubmarineState::default());
    }
}
