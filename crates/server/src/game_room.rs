use std::collections::{HashMap, HashSet};

use shared::{
    CommandId, CommonMeasurements, CrewRole, DiveState, EngineeringMeasurements, MissionConfig,
    PilotMeasurements, PilotOrder, PlayerCommand, PlayerId, ProtocolError, ServerMessage,
    ServerPayload, SubmarineSnapshot,
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
                if !human_roles.contains(&CrewRole::Engineer) {
                    sim.automate_engineer();
                }
                for event in sim.tick(0.05) {
                    broadcast(&players, ServerPayload::Event { tick: sim.tick, event });
                }
                snapshot_id = snapshot_id.wrapping_add(1);
                send_snapshots(&players, &sim, snapshot_id);
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
            PlayerCommand::SetHeading(_)
                | PlayerCommand::SetDepth(_)
                | PlayerCommand::SetSpeed(_)
                | PlayerCommand::SetBallast(_)
                | PlayerCommand::EmergencySurface
        ) | (CrewRole::Sonar, PlayerCommand::SonarPing)
            | (
                CrewRole::Engineer,
                PlayerCommand::RepairSystem(_)
                    | PlayerCommand::SetDiesels(_)
                    | PlayerCommand::SetElectricMotors(_)
                    | PlayerCommand::SetVentilation(_)
                    | PlayerCommand::SetBatteryCharging(_)
            )
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

fn send_snapshots(
    players: &HashMap<PlayerId, (CrewRole, mpsc::Sender<ServerMessage>)>,
    sim: &Simulation,
    snapshot_id: u64,
) {
    for (role, tx) in players.values() {
        let _ = tx.try_send(ServerMessage::new(ServerPayload::Snapshot {
            snapshot_id,
            tick: sim.tick,
            submarine: project_submarine(sim, *role),
        }));
    }
}

fn project_submarine(sim: &Simulation, role: CrewRole) -> SubmarineSnapshot {
    let state = &sim.state;
    let parameters = sim.config.submarine;
    let common = CommonMeasurements {
        x: state.x,
        y: state.y,
        heading: state.heading,
        speed: state.speed,
        depth: state.depth,
        dive_state: state.dive_state,
        acoustic_level: state.acoustic_level,
        alerts: sim.active_alerts(),
    };
    let pilot = (role == CrewRole::Pilot).then_some(PilotMeasurements {
        ordered_heading: state.ordered_heading,
        ordered_speed: state.ordered_speed,
        ordered_depth: state.ordered_depth,
        turn_rate: state.turn_rate,
        vertical_speed: state.vertical_speed,
        ballast: state.ballast,
        emergency_surface: state.emergency_surface,
        max_speed: if state.dive_state == DiveState::Surface {
            parameters.max_surface_speed
        } else {
            parameters.max_submerged_speed
        },
        max_depth: parameters.crush_depth,
    });
    let engineering = (role == CrewRole::Engineer).then_some(EngineeringMeasurements {
        propulsion: state.propulsion,
        battery: state.battery,
        oxygen: state.oxygen,
        electrical_load: state.electrical_load,
        air_intake_available: sim.air_intake_available(),
    });
    SubmarineSnapshot {
        common,
        pilot,
        engineering,
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

        assert_eq!(sim.state.ordered_heading, 80.0);
        assert_eq!(sim.state.ordered_speed, 9.0);
        assert_eq!(sim.state.ordered_depth, 60.0);
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
        let config = MissionConfig::new(44);
        let mut first = Simulation::with_config(config);
        let mut second = Simulation::with_config(config);
        first.apply_command(PlayerCommand::SetSpeed(10.0));

        first.tick(1.0);
        second.tick(1.0);

        assert_ne!(first.state, second.state);
        assert_eq!(second.state.x, 0.0);
        assert_eq!(second.state.y, 0.0);
        assert_eq!(second.state.speed, 0.0);
    }

    #[test]
    fn role_policy_covers_m2_commands() {
        assert!(command_allowed_for_role(
            CrewRole::Pilot,
            &PlayerCommand::SetBallast(shared::BallastState::Flood)
        ));
        assert!(command_allowed_for_role(
            CrewRole::Pilot,
            &PlayerCommand::EmergencySurface
        ));
        assert!(command_allowed_for_role(
            CrewRole::Engineer,
            &PlayerCommand::SetDiesels(true)
        ));
        assert!(command_allowed_for_role(
            CrewRole::Engineer,
            &PlayerCommand::SetBatteryCharging(true)
        ));
        assert!(!command_allowed_for_role(
            CrewRole::Pilot,
            &PlayerCommand::SetDiesels(true)
        ));
        assert!(!command_allowed_for_role(
            CrewRole::Engineer,
            &PlayerCommand::SetDepth(40.0)
        ));
    }

    #[test]
    fn projections_only_include_role_specific_measurements() {
        let sim = Simulation::new();
        let pilot = project_submarine(&sim, CrewRole::Pilot);
        let engineer = project_submarine(&sim, CrewRole::Engineer);
        let sonar = project_submarine(&sim, CrewRole::Sonar);

        assert!(pilot.pilot.is_some());
        assert!(pilot.engineering.is_none());
        assert!(engineer.pilot.is_none());
        assert!(engineer.engineering.is_some());
        assert!(sonar.pilot.is_none());
        assert!(sonar.engineering.is_none());
    }
}
