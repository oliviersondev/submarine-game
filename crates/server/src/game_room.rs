use std::collections::{HashMap, HashSet};

use shared::{
    CommandId, CommonMeasurements, CrewRole, DiveState, EngineeringMeasurements, MissionConfig,
    MissionSnapshot, PilotMeasurements, PilotOrder, PlayerCommand, PlayerId, ProtocolError,
    ServerMessage, ServerPayload, SubmarineSnapshot,
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
                if !human_roles.contains(&CrewRole::Sonar) {
                    sim.automate_sonar();
                }
                for event in sim.automate_weapons(!human_roles.contains(&CrewRole::Weapons), !human_roles.contains(&CrewRole::Captain)) {
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
                if matches!(command, PlayerCommand::OrderWeaponsBot { .. })
                    && human_roles.contains(&CrewRole::Weapons)
                {
                    Err(ProtocolError::WeaponsControlledByHuman)
                } else {
                    sim.apply_command(command).map_err(ProtocolError::from)
                }
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
                sim.apply_pilot_order(order).map_err(ProtocolError::from)
            }
        }
    };

    match result {
        Ok(events) => {
            send_to_player(
                players,
                request.player_id,
                ServerPayload::CommandAccepted {
                    command_id: request.command_id,
                },
            );
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
        ) | (
            CrewRole::Sonar,
            PlayerCommand::SonarPing
                | PlayerCommand::MergeTracks { .. }
                | PlayerCommand::SetTrackShared { .. }
                | PlayerCommand::DropTrack(_)
        ) | (
            CrewRole::Engineer,
            PlayerCommand::RepairSystem(_)
                | PlayerCommand::SetDiesels(_)
                | PlayerCommand::SetElectricMotors(_)
                | PlayerCommand::SetVentilation(_)
                | PlayerCommand::SetBatteryCharging(_)
        ) | (
            CrewRole::Weapons,
            PlayerCommand::FireTorpedo { .. }
                | PlayerCommand::LoadTube(_)
                | PlayerCommand::ConfigureShot { .. }
                | PlayerCommand::FloodTube(_)
                | PlayerCommand::OpenTube(_)
                | PlayerCommand::CloseTube(_)
                | PlayerCommand::PurgeTube(_)
        ) | (
            CrewRole::Captain,
            PlayerCommand::AuthorizeShot { .. } | PlayerCommand::OrderWeaponsBot { .. }
        )
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
    for (role, tx) in players.values() {
        if matches!(
            &message.payload,
            ServerPayload::Event {
                event: shared::GameEvent::SonarPingHeard { .. }
                    | shared::GameEvent::DepthChargeHeard,
                ..
            }
        ) && !matches!(role, CrewRole::Captain | CrewRole::Sonar)
        {
            continue;
        }
        if let ServerPayload::Event {
            event:
                shared::GameEvent::TorpedoFired { .. }
                | shared::GameEvent::TorpedoImpact { .. }
                | shared::GameEvent::TorpedoExpired { .. },
            ..
        } = &message.payload
        {
            if !matches!(
                role,
                CrewRole::Captain | CrewRole::Weapons | CrewRole::Sonar
            ) {
                continue;
            }
        }
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
            mission: project_mission(sim, *role),
        }));
    }
}

fn project_mission(sim: &Simulation, role: CrewRole) -> MissionSnapshot {
    MissionSnapshot {
        submarine: project_submarine(sim, role),
        sonar: (role == CrewRole::Sonar).then(|| sim.sonar_measurements()),
        tactical: matches!(role, CrewRole::Captain | CrewRole::Weapons)
            .then(|| sim.shared_track_measurements()),
        weapons: matches!(role, CrewRole::Captain | CrewRole::Weapons)
            .then(|| sim.weapons_measurements()),
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
        damage: sim.damage_summary(),
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
        compartments: sim.compartments(),
        damage_history: sim.damage_history(),
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
        first.apply_command(PlayerCommand::SetSpeed(10.0)).unwrap();

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
        let pilot = project_mission(&sim, CrewRole::Pilot);
        let engineer = project_mission(&sim, CrewRole::Engineer);
        let sonar = project_mission(&sim, CrewRole::Sonar);
        let captain = project_mission(&sim, CrewRole::Captain);

        assert!(pilot.submarine.pilot.is_some());
        assert!(pilot.submarine.engineering.is_none());
        assert!(pilot.sonar.is_none());
        assert!(pilot.tactical.is_none());
        assert!(engineer.submarine.pilot.is_none());
        assert!(engineer.submarine.engineering.is_some());
        assert!(sonar.submarine.pilot.is_none());
        assert!(sonar.submarine.engineering.is_none());
        assert!(sonar.sonar.is_some());
        assert!(sonar.tactical.is_none());
        assert!(captain.sonar.is_none());
        assert!(captain.tactical.is_some());
        assert!(captain.weapons.is_some());
        assert!(pilot.weapons.is_none());
        assert!(engineer.weapons.is_none());
        assert!(sonar.weapons.is_none());
    }

    #[test]
    fn weapons_procedure_and_captain_authorization_have_distinct_permissions() {
        let settings = shared::ShotSettings::for_target(shared::TrackId(1));
        for command in [
            PlayerCommand::LoadTube(0),
            PlayerCommand::ConfigureShot { tube: 0, settings },
            PlayerCommand::FloodTube(0),
            PlayerCommand::OpenTube(0),
            PlayerCommand::FireTorpedo { tube: 0 },
            PlayerCommand::CloseTube(0),
            PlayerCommand::PurgeTube(0),
        ] {
            for role in CrewRole::ALL {
                assert_eq!(
                    command_allowed_for_role(role, &command),
                    role == CrewRole::Weapons
                );
            }
        }
        for command in [
            PlayerCommand::AuthorizeShot { tube: 0, settings },
            PlayerCommand::OrderWeaponsBot {
                target: settings.target,
            },
        ] {
            for role in CrewRole::ALL {
                assert_eq!(
                    command_allowed_for_role(role, &command),
                    role == CrewRole::Captain
                );
            }
        }
    }

    #[test]
    fn weapons_bot_order_cannot_override_a_human() {
        let (tx, mut rx) = mpsc::channel(2);
        let players = HashMap::from([(PlayerId(1), (CrewRole::Captain, tx))]);
        let humans = HashSet::from([CrewRole::Captain, CrewRole::Weapons]);
        let mut sim = Simulation::new();
        handle_command(
            &mut sim,
            &players,
            &humans,
            request(
                CrewRole::Captain,
                GameRoomAction::Player(PlayerCommand::OrderWeaponsBot {
                    target: shared::TrackId(1),
                }),
            ),
        );
        assert_eq!(sim.weapons_measurements().reserve, 5);
        assert!(matches!(
            rx.try_recv().unwrap().payload,
            ServerPayload::Error {
                command_id: Some(CommandId(7)),
                error: ProtocolError::WeaponsControlledByHuman
            }
        ));
    }

    #[test]
    fn accepted_commands_are_acknowledged_only_to_the_sender() {
        let (tx, mut rx) = mpsc::channel(2);
        let (other_tx, mut other_rx) = mpsc::channel(2);
        let players = HashMap::from([
            (PlayerId(1), (CrewRole::Weapons, tx)),
            (PlayerId(2), (CrewRole::Captain, other_tx)),
        ]);
        let mut sim = Simulation::new();
        handle_command(
            &mut sim,
            &players,
            &HashSet::from([CrewRole::Weapons]),
            request(
                CrewRole::Weapons,
                GameRoomAction::Player(PlayerCommand::LoadTube(0)),
            ),
        );
        assert!(matches!(
            rx.try_recv().unwrap().payload,
            ServerPayload::CommandAccepted {
                command_id: CommandId(7)
            }
        ));
        assert!(other_rx.try_recv().is_err());
        assert_eq!(sim.weapons_measurements().reserve, 4);
    }

    #[test]
    fn torpedo_observations_are_filtered_and_forbidden_commands_do_not_mutate_state() {
        let mut players = HashMap::new();
        let mut receivers = vec![];
        for (index, role) in CrewRole::ALL.into_iter().enumerate() {
            let (tx, rx) = mpsc::channel(4);
            players.insert(PlayerId(index as u64 + 1), (role, tx));
            receivers.push((role, rx));
        }
        for event in [
            shared::GameEvent::TorpedoFired { bearing: 5.0 },
            shared::GameEvent::TorpedoImpact { torpedo_id: 1 },
            shared::GameEvent::TorpedoExpired { torpedo_id: 2 },
        ] {
            broadcast(&players, ServerPayload::Event { tick: 1, event });
            for (role, rx) in &mut receivers {
                assert_eq!(
                    rx.try_recv().is_ok(),
                    matches!(
                        role,
                        CrewRole::Captain | CrewRole::Weapons | CrewRole::Sonar
                    )
                );
            }
        }
        let mut sim = Simulation::new();
        handle_command(
            &mut sim,
            &players,
            &HashSet::new(),
            request(
                CrewRole::Captain,
                GameRoomAction::Player(PlayerCommand::LoadTube(0)),
            ),
        );
        assert_eq!(sim.weapons_measurements().reserve, 5);
        for (role, rx) in &mut receivers {
            if *role == CrewRole::Captain {
                assert!(matches!(
                    rx.try_recv().unwrap().payload,
                    ServerPayload::Error {
                        command_id: Some(CommandId(7)),
                        error: ProtocolError::CommandNotAllowedForRole
                    }
                ));
            } else {
                assert!(rx.try_recv().is_err());
            }
        }
    }

    #[test]
    fn sonar_track_commands_are_restricted_to_sonar_role() {
        let command = PlayerCommand::SetTrackShared {
            track_id: shared::TrackId(1),
            shared: true,
        };
        assert!(command_allowed_for_role(CrewRole::Sonar, &command));
        assert!(!command_allowed_for_role(CrewRole::Captain, &command));
        assert!(!command_allowed_for_role(CrewRole::Weapons, &command));
    }

    #[test]
    fn damage_snapshots_survive_missed_events_and_details_are_engineer_only() {
        let mut sim = Simulation::new();
        // Real acoustic acquisition and combat; no injected enemy decision or damage.
        sim.state.x = 0.0; // escort starts within 400 m of this point
        sim.state.y = 4_050.0;
        sim.state.depth = 100.0;
        sim.state.ordered_depth = 100.0;
        sim.apply_command(PlayerCommand::SonarPing).unwrap();
        for _ in 0..3_600 {
            sim.tick(0.05);
        }
        assert!(sim.damage_summary().hull_integrity < 100.0);
        for role in CrewRole::ALL {
            let projection = project_mission(&sim, role);
            assert_eq!(projection.submarine.common.damage, sim.damage_summary());
            assert!(projection
                .submarine
                .common
                .alerts
                .contains(&shared::AlertKind::HullDamaged));
            if let Some(engineering) = projection.submarine.engineering {
                assert_eq!(role, CrewRole::Engineer);
                assert!(!engineering.damage_history.is_empty());
                assert_eq!(engineering.compartments, sim.compartments());
            } else {
                assert_ne!(role, CrewRole::Engineer);
            }
            let message = ServerMessage::new(ServerPayload::Snapshot {
                snapshot_id: 1,
                tick: sim.tick,
                mission: project_mission(&sim, role),
            });
            assert_eq!(
                shared::codec::decode::<ServerMessage>(&shared::codec::encode(&message)).unwrap(),
                message
            );
        }
    }

    #[test]
    fn charge_sounds_are_filtered_but_local_damage_reaches_all_roles() {
        let mut players = HashMap::new();
        let mut receivers = vec![];
        for (index, role) in CrewRole::ALL.into_iter().enumerate() {
            let (tx, rx) = mpsc::channel(4);
            players.insert(PlayerId(index as u64), (role, tx));
            receivers.push((role, rx));
        }
        broadcast(
            &players,
            ServerPayload::Event {
                tick: 1,
                event: shared::GameEvent::DepthChargeHeard,
            },
        );
        for (role, rx) in &mut receivers {
            assert_eq!(
                rx.try_recv().is_ok(),
                matches!(role, CrewRole::Captain | CrewRole::Sonar)
            );
        }
        broadcast(
            &players,
            ServerPayload::Event {
                tick: 2,
                event: shared::GameEvent::SubmarineDamaged {
                    cause: shared::DamageCause::DepthCharge,
                },
            },
        );
        for (_, rx) in &mut receivers {
            assert!(rx.try_recv().is_ok());
        }
    }

    #[test]
    fn audible_pings_are_only_sent_to_captain_and_sonar() {
        let mut players = HashMap::new();
        let mut receivers = vec![];
        for (index, role) in CrewRole::ALL.into_iter().enumerate() {
            let (tx, rx) = mpsc::channel(4);
            players.insert(PlayerId(index as u64), (role, tx));
            receivers.push((role, rx));
        }
        broadcast(
            &players,
            ServerPayload::Event {
                tick: 1,
                event: shared::GameEvent::SonarPingHeard {
                    bearing: 45.0,
                    bearing_uncertainty: 3.0,
                },
            },
        );
        for (role, mut rx) in receivers {
            assert_eq!(
                rx.try_recv().is_ok(),
                matches!(role, CrewRole::Captain | CrewRole::Sonar)
            );
        }
    }
}
