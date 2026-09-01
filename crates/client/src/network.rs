use bevy::prelude::*;
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use shared::{
    codec::{decode, encode},
    ClientMessage, ClientPayload, CommandId, CrewRole, GameEvent, LobbyCommand, LobbySnapshot,
    MissionCommand, PilotOrder, PlayerCommand, PlayerId, ProtocolError, RoomId, ServerMessage,
    ServerPayload, SessionId, SonarMeasurements, SubmarineSnapshot, SystemId, TacticalMeasurements,
    PROTOCOL_VERSION,
};

use crate::role::role_from_environment;

pub struct WsConnection {
    pub sender: WsSender,
    pub receiver: WsReceiver,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct NetworkReceiveSet;

#[derive(Debug, Clone)]
pub enum RoomRequest {
    Create,
    Join(RoomId),
}

#[derive(Resource, Default)]
pub struct CommandQueue {
    mission: Vec<MissionCommand>,
    lobby: Vec<LobbyCommand>,
    next_id: u64,
    heading_intent: Option<f32>,
    speed_intent: Option<f32>,
    depth_intent: Option<f32>,
    diesels_intent: Option<bool>,
    electric_motors_intent: Option<bool>,
    ventilation_intent: Option<bool>,
    charging_intent: Option<bool>,
}

impl CommandQueue {
    pub fn push(&mut self, command: PlayerCommand) {
        match &command {
            PlayerCommand::SetHeading(value) => self.heading_intent = Some(*value),
            PlayerCommand::SetSpeed(value) => self.speed_intent = Some(*value),
            PlayerCommand::SetDepth(value) => self.depth_intent = Some(*value),
            PlayerCommand::SetDiesels(value) => self.diesels_intent = Some(*value),
            PlayerCommand::SetElectricMotors(value) => self.electric_motors_intent = Some(*value),
            PlayerCommand::SetVentilation(value) => self.ventilation_intent = Some(*value),
            PlayerCommand::SetBatteryCharging(value) => self.charging_intent = Some(*value),
            _ => {}
        }
        let command_id = self.next_command_id();
        self.mission.push(MissionCommand::Player {
            command_id,
            command,
        });
    }

    pub fn order_pilot(&mut self, order: PilotOrder) {
        let command_id = self.next_command_id();
        self.mission
            .push(MissionCommand::OrderPilotBot { command_id, order });
    }

    pub fn lobby(&mut self, command: LobbyCommand) {
        self.lobby.push(command);
    }

    pub fn pending_heading(&self, fallback: f32) -> f32 {
        self.heading_intent.unwrap_or(fallback)
    }

    pub fn pending_speed(&self, fallback: f32) -> f32 {
        self.speed_intent.unwrap_or(fallback)
    }

    pub fn pending_depth(&self, fallback: f32) -> f32 {
        self.depth_intent.unwrap_or(fallback)
    }

    pub fn pending_diesels(&self, fallback: bool) -> bool {
        self.diesels_intent.unwrap_or(fallback)
    }

    pub fn pending_electric_motors(&self, fallback: bool) -> bool {
        self.electric_motors_intent.unwrap_or(fallback)
    }

    pub fn pending_ventilation(&self, fallback: bool) -> bool {
        self.ventilation_intent.unwrap_or(fallback)
    }

    pub fn pending_charging(&self, fallback: bool) -> bool {
        self.charging_intent.unwrap_or(fallback)
    }

    fn reconcile(&mut self, snapshot: &SubmarineSnapshot) {
        if let Some(pilot) = &snapshot.pilot {
            clear_matching_float(&mut self.heading_intent, pilot.ordered_heading);
            clear_matching_float(&mut self.speed_intent, pilot.ordered_speed);
            clear_matching_float(&mut self.depth_intent, pilot.ordered_depth);
        }
        if let Some(engineering) = &snapshot.engineering {
            if !engineering.air_intake_available {
                self.diesels_intent = None;
                self.ventilation_intent = None;
            }
            clear_matching(&mut self.diesels_intent, engineering.propulsion.diesels_on);
            clear_matching(
                &mut self.electric_motors_intent,
                engineering.propulsion.electric_motors_on,
            );
            clear_matching(
                &mut self.ventilation_intent,
                engineering.propulsion.ventilation_on,
            );
            clear_matching(&mut self.charging_intent, engineering.propulsion.charging);
        }
    }

    fn clear_intents(&mut self) {
        self.heading_intent = None;
        self.speed_intent = None;
        self.depth_intent = None;
        self.diesels_intent = None;
        self.electric_motors_intent = None;
        self.ventilation_intent = None;
        self.charging_intent = None;
    }

    fn next_command_id(&mut self) -> CommandId {
        self.next_id = self.next_id.wrapping_add(1);
        CommandId(self.next_id)
    }
}

fn clear_matching<T: PartialEq + Copy>(intent: &mut Option<T>, authoritative: T) {
    if intent.is_some_and(|value| value == authoritative) {
        *intent = None;
    }
}

fn clear_matching_float(intent: &mut Option<f32>, authoritative: f32) {
    if intent.is_some_and(|value| (value - authoritative).abs() < f32::EPSILON) {
        *intent = None;
    }
}

#[derive(Resource, Default)]
pub struct LocalPlayer {
    pub role: Option<CrewRole>,
    pub room_code: String,
    pub request: Option<RoomRequest>,
    pub session_id: Option<SessionId>,
    pub id: Option<PlayerId>,
    pub joined: bool,
    connection_started: bool,
}

#[derive(Resource, Default)]
pub struct GameState {
    pub lobby: Option<LobbySnapshot>,
    pub previous_submarine: Option<SubmarineSnapshot>,
    pub submarine: Option<SubmarineSnapshot>,
    pub sonar: Option<SonarMeasurements>,
    pub tactical: Option<TacticalMeasurements>,
    pub snapshot_id: u64,
    pub server_tick: u64,
    pub game_started: bool,
    pub last_error: Option<ProtocolError>,
    pub latest_event: Option<GameEvent>,
}

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        let role = role_from_environment();
        app.insert_resource(LocalPlayer {
            role,
            request: role.map(|_| RoomRequest::Create),
            ..default()
        })
        .insert_resource(GameState::default())
        .init_resource::<CommandQueue>()
        .add_systems(Update, manage_connection)
        .add_systems(
            Update,
            poll_messages
                .after(manage_connection)
                .in_set(NetworkReceiveSet),
        )
        .add_systems(Update, queue_keyboard_command.after(poll_messages))
        .add_systems(PostUpdate, flush_commands);
    }
}

fn manage_connection(world: &mut World) {
    let should_connect = {
        let player = world.resource::<LocalPlayer>();
        player.role.is_some() && player.request.is_some()
    };
    let has_connection = world.get_non_send::<WsConnection>().is_some();

    if !should_connect {
        if has_connection {
            world.remove_non_send::<WsConnection>();
        }
        let mut player = world.resource_mut::<LocalPlayer>();
        player.connection_started = false;
        player.joined = false;
        return;
    }
    if has_connection && !world.resource::<LocalPlayer>().connection_started {
        world.remove_non_send::<WsConnection>();
        return;
    }
    if has_connection || world.resource::<LocalPlayer>().connection_started {
        return;
    }

    world.resource_mut::<LocalPlayer>().connection_started = true;
    match ewebsock::connect(&websocket_url(), ewebsock::Options::default()) {
        Ok((sender, receiver)) => world.insert_non_send(WsConnection { sender, receiver }),
        Err(error) => {
            let mut player = world.resource_mut::<LocalPlayer>();
            player.connection_started = false;
            player.request = None;
            world.resource_mut::<GameState>().last_error = Some(ProtocolError::ConnectionFailed);
            error!("WebSocket connect failed: {error}");
        }
    }
}

fn poll_messages(
    ws: Option<NonSendMut<WsConnection>>,
    mut player: ResMut<LocalPlayer>,
    mut state: ResMut<GameState>,
    mut commands: ResMut<CommandQueue>,
) {
    let Some(mut ws) = ws else { return };

    while let Some(event) = ws.receiver.try_recv() {
        match event {
            WsEvent::Opened => {
                if player.joined {
                    continue;
                }
                let Some(role) = player.role else { continue };
                let Some(request) = player.request.clone() else {
                    continue;
                };
                let command = match request {
                    RoomRequest::Create => LobbyCommand::CreateRoom { role },
                    RoomRequest::Join(room_id) => LobbyCommand::JoinRoom { room_id, role },
                };
                ws.sender.send(WsMessage::Binary(encode(&ClientMessage::new(
                    ClientPayload::Lobby(command),
                ))));
                player.joined = true;
            }
            WsEvent::Message(WsMessage::Binary(bytes)) => match decode::<ServerMessage>(&bytes) {
                Ok(message) if message.version == PROTOCOL_VERSION => {
                    handle_server_message(message.payload, &mut player, &mut state, &mut commands)
                }
                Ok(message) => {
                    state.last_error = Some(ProtocolError::IncompatibleVersion {
                        expected: PROTOCOL_VERSION,
                        received: message.version,
                    });
                }
                Err(error) => warn!("Invalid server message: {error}"),
            },
            WsEvent::Closed | WsEvent::Error(_) => {
                handle_connection_end(&mut player, &mut state);
            }
            _ => {}
        }
    }
}

fn handle_connection_end(player: &mut LocalPlayer, state: &mut GameState) {
    player.joined = false;
    player.connection_started = false;
    state.game_started = false;
    if player.id.is_none() {
        player.request = None;
        if state.last_error.is_none() {
            state.last_error = Some(ProtocolError::ConnectionFailed);
        }
    }
}

fn queue_keyboard_command(
    keyboard: Res<ButtonInput<KeyCode>>,
    player: Res<LocalPlayer>,
    state: Res<GameState>,
    mut commands: ResMut<CommandQueue>,
) {
    if !state.game_started {
        return;
    }
    let Some(role) = player.role else { return };
    if let Some(command) = command_for_input(role, &keyboard, &state, &commands) {
        commands.push(command);
    }
}

fn flush_commands(mut commands: ResMut<CommandQueue>, ws: Option<NonSendMut<WsConnection>>) {
    let Some(mut ws) = ws else {
        commands.mission.clear();
        commands.lobby.clear();
        return;
    };

    for command in commands.lobby.drain(..) {
        ws.sender.send(WsMessage::Binary(encode(&ClientMessage::new(
            ClientPayload::Lobby(command),
        ))));
    }
    for command in commands.mission.drain(..) {
        ws.sender.send(WsMessage::Binary(encode(&ClientMessage::new(
            ClientPayload::Mission(command),
        ))));
    }
}

fn command_for_input(
    role: CrewRole,
    keyboard: &ButtonInput<KeyCode>,
    state: &GameState,
    commands: &CommandQueue,
) -> Option<PlayerCommand> {
    match role {
        CrewRole::Captain => None,
        CrewRole::Pilot => {
            let submarine = state.submarine.as_ref()?;
            let pilot = submarine.pilot.as_ref()?;
            if keyboard.just_pressed(KeyCode::ArrowLeft) {
                Some(PlayerCommand::SetHeading(
                    (commands.pending_heading(pilot.ordered_heading) - 5.0).rem_euclid(360.0),
                ))
            } else if keyboard.just_pressed(KeyCode::ArrowRight) {
                Some(PlayerCommand::SetHeading(
                    (commands.pending_heading(pilot.ordered_heading) + 5.0).rem_euclid(360.0),
                ))
            } else if keyboard.just_pressed(KeyCode::ArrowUp) {
                Some(PlayerCommand::SetSpeed(
                    (commands.pending_speed(pilot.ordered_speed) + 1.0).min(18.0),
                ))
            } else if keyboard.just_pressed(KeyCode::ArrowDown) {
                Some(PlayerCommand::SetSpeed(
                    (commands.pending_speed(pilot.ordered_speed) - 1.0).max(0.0),
                ))
            } else if keyboard.just_pressed(KeyCode::PageDown) {
                Some(PlayerCommand::SetDepth(
                    (commands.pending_depth(pilot.ordered_depth) + 10.0).min(pilot.max_depth),
                ))
            } else if keyboard.just_pressed(KeyCode::PageUp) {
                Some(PlayerCommand::SetDepth(
                    (commands.pending_depth(pilot.ordered_depth) - 10.0).max(0.0),
                ))
            } else {
                None
            }
        }
        CrewRole::Sonar => keyboard
            .just_pressed(KeyCode::Space)
            .then_some(PlayerCommand::SonarPing),
        CrewRole::Engineer => {
            let engineering = state.submarine.as_ref()?.engineering.as_ref()?;
            if keyboard.just_pressed(KeyCode::Digit1) {
                Some(PlayerCommand::SetDiesels(
                    !commands.pending_diesels(engineering.propulsion.diesels_on),
                ))
            } else if keyboard.just_pressed(KeyCode::Digit2) {
                Some(PlayerCommand::SetElectricMotors(
                    !commands.pending_electric_motors(engineering.propulsion.electric_motors_on),
                ))
            } else if keyboard.just_pressed(KeyCode::Digit3) {
                Some(PlayerCommand::SetVentilation(
                    !commands.pending_ventilation(engineering.propulsion.ventilation_on),
                ))
            } else if keyboard.just_pressed(KeyCode::Digit4) {
                Some(PlayerCommand::SetBatteryCharging(
                    !commands.pending_charging(engineering.propulsion.charging),
                ))
            } else if keyboard.just_pressed(KeyCode::Digit5) {
                Some(PlayerCommand::RepairSystem(SystemId::Engine))
            } else {
                None
            }
        }
        CrewRole::Weapons => {
            let submarine = state.submarine.as_ref()?;
            keyboard
                .just_pressed(KeyCode::Space)
                .then_some(PlayerCommand::FireTorpedo {
                    bearing: submarine.common.heading,
                })
        }
    }
}

fn handle_server_message(
    payload: ServerPayload,
    player: &mut LocalPlayer,
    state: &mut GameState,
    commands: &mut CommandQueue,
) {
    match payload {
        ServerPayload::SessionJoined {
            session_id,
            player_id,
            room_id,
            role,
        } => {
            player.session_id = Some(session_id);
            player.id = Some(player_id);
            player.room_code = room_id.0;
            player.role = Some(role);
            commands.clear_intents();
        }
        ServerPayload::Lobby(lobby) => state.lobby = Some(lobby),
        ServerPayload::MissionStarted { .. } => {
            state.game_started = true;
            commands.clear_intents();
        }
        ServerPayload::Snapshot {
            snapshot_id,
            tick,
            mission,
        } => {
            if snapshot_id > state.snapshot_id {
                commands.reconcile(&mission.submarine);
                state.previous_submarine = state.submarine.replace(mission.submarine);
                state.sonar = mission.sonar;
                state.tactical = mission.tactical;
                state.snapshot_id = snapshot_id;
                state.server_tick = tick;
            }
        }
        ServerPayload::Event { tick, event } => {
            state.server_tick = state.server_tick.max(tick);
            debug!("GameEvent: {event:?}");
            state.latest_event = Some(event);
        }
        ServerPayload::Error { error, .. } => {
            commands.clear_intents();
            if matches!(
                error,
                ProtocolError::RoomNotFound
                    | ProtocolError::RoomAlreadyStarted
                    | ProtocolError::RoleAlreadyTaken(_)
            ) {
                player.request = None;
                player.id = None;
                player.session_id = None;
            }
            state.last_error = Some(error);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn websocket_url() -> String {
    let Some(location) = web_sys::window().map(|window| window.location()) else {
        return "ws://127.0.0.1:3000/ws".to_owned();
    };
    let protocol = if location.protocol().ok().as_deref() == Some("https:") {
        "wss"
    } else {
        "ws"
    };
    let host = location.hostname().unwrap_or_else(|_| "127.0.0.1".into());
    let port = match location.port().ok().as_deref() {
        Some("8080") => ":3000",
        Some("") | None => "",
        Some(port) => return format!("{protocol}://{host}:{port}/ws"),
    };
    format!("{protocol}://{host}{port}/ws")
}

#[cfg(not(target_arch = "wasm32"))]
fn websocket_url() -> String {
    std::env::var("WS_URL").unwrap_or_else(|_| "ws://127.0.0.1:3000/ws".to_owned())
}

pub fn controls_for_role(role: CrewRole) -> &'static str {
    match role {
        CrewRole::Captain => "ordre tactile vers le bot Pilote",
        CrewRole::Pilot => "Gauche/Droite : cap\nHaut/Bas : vitesse\nPgUp/PgDown : profondeur",
        CrewRole::Sonar => {
            "Espace/PING : sonar actif\nPistes : selection, partage, fusion, abandon"
        }
        CrewRole::Engineer => {
            "1 diesels | 2 electrique | 3 ventilation | 4 recharge | 5 reparation"
        }
        CrewRole::Weapons => "Pistes partagees en lecture seule\nEspace : tir dans le cap actuel",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_queue_assigns_monotonic_ids() {
        let mut queue = CommandQueue::default();
        queue.push(PlayerCommand::SetSpeed(1.0));
        queue.order_pilot(PilotOrder {
            heading: 10.0,
            speed: 2.0,
            depth: 3.0,
        });

        assert!(matches!(
            queue.mission[0],
            MissionCommand::Player {
                command_id: CommandId(1),
                ..
            }
        ));
        assert!(matches!(
            queue.mission[1],
            MissionCommand::OrderPilotBot {
                command_id: CommandId(2),
                ..
            }
        ));
    }

    #[test]
    fn unavailable_air_intake_clears_impossible_engineering_intents() {
        let mut queue = CommandQueue::default();
        queue.push(PlayerCommand::SetDiesels(true));
        queue.push(PlayerCommand::SetVentilation(true));
        let snapshot = SubmarineSnapshot {
            common: shared::CommonMeasurements {
                x: 0.0,
                y: 0.0,
                heading: 0.0,
                speed: 0.0,
                depth: 40.0,
                dive_state: shared::DiveState::Submerged,
                acoustic_level: shared::AcousticLevel::Silent,
                alerts: vec![],
            },
            pilot: None,
            engineering: Some(shared::EngineeringMeasurements {
                propulsion: shared::PropulsionState {
                    diesels_on: false,
                    electric_motors_on: true,
                    ventilation_on: false,
                    charging: false,
                },
                battery: 80.0,
                oxygen: 80.0,
                electrical_load: 0.1,
                air_intake_available: false,
            }),
        };

        queue.reconcile(&snapshot);

        assert!(!queue.pending_diesels(false));
        assert!(!queue.pending_ventilation(false));
    }

    #[test]
    fn room_not_found_keeps_player_in_setup_for_retry() {
        let mut player = LocalPlayer {
            role: Some(CrewRole::Pilot),
            request: Some(RoomRequest::Join(RoomId("999999".to_owned()))),
            joined: true,
            connection_started: true,
            ..default()
        };
        let mut state = GameState::default();
        let mut commands = CommandQueue::default();

        handle_server_message(
            ServerPayload::Error {
                command_id: None,
                error: ProtocolError::RoomNotFound,
            },
            &mut player,
            &mut state,
            &mut commands,
        );
        handle_connection_end(&mut player, &mut state);

        assert!(player.request.is_none());
        assert!(player.id.is_none());
        assert_eq!(state.last_error, Some(ProtocolError::RoomNotFound));
    }

    #[test]
    fn role_taken_error_survives_the_server_closing_the_socket() {
        let mut player = LocalPlayer {
            role: Some(CrewRole::Pilot),
            request: Some(RoomRequest::Join(RoomId("000004".to_owned()))),
            joined: true,
            connection_started: true,
            ..default()
        };
        let mut state = GameState::default();
        let mut commands = CommandQueue::default();

        handle_server_message(
            ServerPayload::Error {
                command_id: None,
                error: ProtocolError::RoleAlreadyTaken(CrewRole::Pilot),
            },
            &mut player,
            &mut state,
            &mut commands,
        );
        handle_connection_end(&mut player, &mut state);

        assert!(player.request.is_none());
        assert_eq!(
            state.last_error,
            Some(ProtocolError::RoleAlreadyTaken(CrewRole::Pilot))
        );
    }
}
