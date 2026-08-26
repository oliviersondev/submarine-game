use bevy::prelude::*;
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use shared::{
    codec::{decode, encode},
    ClientMessage, CrewRole, GameEvent, PlayerCommand, ProtocolError, ServerMessage,
    SubmarineState, SystemId,
};

use crate::role::current_role;

// WsSender/WsReceiver use Rc<WebSocket> in WASM — not Send+Sync.
// Stored as a NonSend resource so Bevy keeps it on the main thread.
pub struct WsConnection {
    pub sender: WsSender,
    pub receiver: WsReceiver,
}

#[derive(Resource)]
pub struct LocalPlayer {
    pub role: CrewRole,
    pub id: Option<u32>,
    pub joined: bool,
}

#[derive(Resource, Default)]
pub struct GameState {
    pub previous_submarine: Option<SubmarineState>,
    pub submarine: Option<SubmarineState>,
    pub snapshot_id: u64,
    pub game_started: bool,
    pub last_error: Option<ProtocolError>,
}

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LocalPlayer {
            role: current_role(),
            id: None,
            joined: false,
        })
        .insert_resource(GameState::default())
        .add_systems(Update, (poll_messages, send_keyboard_command).chain());

        match ewebsock::connect("ws://127.0.0.1:3000/ws", ewebsock::Options::default()) {
            Ok((sender, receiver)) => {
                app.insert_non_send(WsConnection { sender, receiver });
            }
            Err(e) => error!("WebSocket connect failed: {e}"),
        }
    }
}

fn poll_messages(
    ws: Option<NonSendMut<WsConnection>>,
    mut player: ResMut<LocalPlayer>,
    mut state: ResMut<GameState>,
) {
    let Some(mut ws) = ws else { return };

    while let Some(event) = ws.receiver.try_recv() {
        match event {
            WsEvent::Opened => {
                if !player.joined {
                    let msg = encode(&ClientMessage::JoinLobby { role: player.role });
                    ws.sender.send(WsMessage::Binary(msg));
                    player.joined = true;
                    info!("WS opened — JoinLobby sent as {:?}", player.role);
                }
            }
            WsEvent::Message(WsMessage::Binary(bytes)) => {
                if let Ok(msg) = decode::<ServerMessage>(&bytes) {
                    handle_server_message(msg, &mut player, &mut state);
                }
            }
            WsEvent::Closed => info!("WebSocket closed"),
            WsEvent::Error(e) => error!("WS error: {e}"),
            _ => {}
        }
    }
}

fn send_keyboard_command(
    keyboard: Res<ButtonInput<KeyCode>>,
    player: Res<LocalPlayer>,
    state: Res<GameState>,
    ws: Option<NonSendMut<WsConnection>>,
) {
    if !state.game_started {
        return;
    }

    let Some(command) = command_for_input(player.role, &keyboard, &state) else {
        return;
    };
    let Some(mut ws) = ws else { return };

    info!("Command sent: {command:?}");
    ws.sender
        .send(WsMessage::Binary(encode(&ClientMessage::Command(command))));
}

fn command_for_input(
    role: CrewRole,
    keyboard: &ButtonInput<KeyCode>,
    state: &GameState,
) -> Option<PlayerCommand> {
    match role {
        CrewRole::Captain => None,
        CrewRole::Pilot => {
            let submarine = state.submarine.as_ref()?;
            if keyboard.just_pressed(KeyCode::ArrowLeft) {
                Some(PlayerCommand::SetHeading(
                    (submarine.heading - 5.0).rem_euclid(360.0),
                ))
            } else if keyboard.just_pressed(KeyCode::ArrowRight) {
                Some(PlayerCommand::SetHeading(
                    (submarine.heading + 5.0).rem_euclid(360.0),
                ))
            } else if keyboard.just_pressed(KeyCode::ArrowUp) {
                Some(PlayerCommand::SetSpeed((submarine.speed + 1.0).min(20.0)))
            } else if keyboard.just_pressed(KeyCode::ArrowDown) {
                Some(PlayerCommand::SetSpeed((submarine.speed - 1.0).max(0.0)))
            } else if keyboard.just_pressed(KeyCode::PageDown) {
                Some(PlayerCommand::SetDepth(
                    (submarine.depth + 10.0).min(1_000.0),
                ))
            } else if keyboard.just_pressed(KeyCode::PageUp) {
                Some(PlayerCommand::SetDepth((submarine.depth - 10.0).max(0.0)))
            } else {
                None
            }
        }
        CrewRole::Sonar => keyboard
            .just_pressed(KeyCode::Space)
            .then_some(PlayerCommand::SonarPing),
        CrewRole::Engineer => {
            if keyboard.just_pressed(KeyCode::Digit1) {
                Some(PlayerCommand::RepairSystem(SystemId::Engine))
            } else if keyboard.just_pressed(KeyCode::Digit2) {
                Some(PlayerCommand::RepairSystem(SystemId::Torpedo))
            } else if keyboard.just_pressed(KeyCode::Digit3) {
                Some(PlayerCommand::RepairSystem(SystemId::Sonar))
            } else if keyboard.just_pressed(KeyCode::Digit4) {
                Some(PlayerCommand::RepairSystem(SystemId::Life))
            } else if keyboard.just_pressed(KeyCode::Digit5) {
                Some(PlayerCommand::RepairSystem(SystemId::Navigation))
            } else {
                None
            }
        }
        CrewRole::Weapons => {
            let submarine = state.submarine.as_ref()?;
            keyboard
                .just_pressed(KeyCode::Space)
                .then_some(PlayerCommand::FireTorpedo {
                    bearing: submarine.heading,
                })
        }
    }
}

fn handle_server_message(msg: ServerMessage, player: &mut LocalPlayer, state: &mut GameState) {
    match msg {
        ServerMessage::JoinAck { player_id, role } => {
            player.id = Some(player_id);
            player.role = role;
            info!("JoinAck: id={player_id}, role={role:?}");
            log_controls(role);
        }
        ServerMessage::GameStarted => {
            state.game_started = true;
            info!("Game started!");
        }
        ServerMessage::Event(GameEvent::StateSnapshot(sub)) => {
            state.previous_submarine = state.submarine.replace(sub);
            state.snapshot_id = state.snapshot_id.wrapping_add(1);
        }
        ServerMessage::Event(event) => {
            debug!("GameEvent: {event:?}");
        }
        ServerMessage::Error(e) => {
            warn!("Server error: {e:?}");
            state.last_error = Some(e);
        }
    }
}

fn log_controls(role: CrewRole) {
    info!("Controls: {}", controls_for_role(role));
}

pub fn controls_for_role(role: CrewRole) -> &'static str {
    match role {
        CrewRole::Captain => "aucune commande pour le moment",
        CrewRole::Pilot => "Gauche/Droite : cap\nHaut/Bas : vitesse\nPgUp/PgDown : profondeur",
        CrewRole::Sonar => "Espace : ping sonar",
        CrewRole::Engineer => "1 moteur | 2 torpille | 3 sonar | 4 survie | 5 navigation",
        CrewRole::Weapons => "Espace : tir dans le cap actuel",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_state() -> GameState {
        GameState {
            submarine: Some(SubmarineState {
                heading: 0.0,
                speed: 0.0,
                depth: 0.0,
                ..default()
            }),
            game_started: true,
            ..default()
        }
    }

    #[test]
    fn pilot_controls_wrap_heading_and_clamp_speed_and_depth() {
        let mut keyboard = ButtonInput::default();
        keyboard.press(KeyCode::ArrowLeft);
        assert!(matches!(
            command_for_input(CrewRole::Pilot, &keyboard, &game_state()),
            Some(PlayerCommand::SetHeading(355.0))
        ));

        let mut keyboard = ButtonInput::default();
        keyboard.press(KeyCode::ArrowDown);
        assert!(matches!(
            command_for_input(CrewRole::Pilot, &keyboard, &game_state()),
            Some(PlayerCommand::SetSpeed(0.0))
        ));

        let mut keyboard = ButtonInput::default();
        keyboard.press(KeyCode::PageUp);
        assert!(matches!(
            command_for_input(CrewRole::Pilot, &keyboard, &game_state()),
            Some(PlayerCommand::SetDepth(0.0))
        ));
    }

    #[test]
    fn station_controls_create_role_specific_commands() {
        let mut keyboard = ButtonInput::default();
        keyboard.press(KeyCode::Space);
        assert!(matches!(
            command_for_input(CrewRole::Sonar, &keyboard, &game_state()),
            Some(PlayerCommand::SonarPing)
        ));
        assert!(matches!(
            command_for_input(CrewRole::Weapons, &keyboard, &game_state()),
            Some(PlayerCommand::FireTorpedo { bearing: 0.0 })
        ));

        let mut keyboard = ButtonInput::default();
        keyboard.press(KeyCode::Digit3);
        assert!(matches!(
            command_for_input(CrewRole::Engineer, &keyboard, &game_state()),
            Some(PlayerCommand::RepairSystem(SystemId::Sonar))
        ));
    }
}
