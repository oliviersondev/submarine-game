use bevy::prelude::*;
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use shared::{
    codec::{decode, encode},
    ClientMessage, CrewRole, GameEvent, ServerMessage, SubmarineState,
};

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
    pub submarine: Option<SubmarineState>,
    pub game_started: bool,
}

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LocalPlayer {
            role: CrewRole::Captain,
            id: None,
            joined: false,
        })
        .insert_resource(GameState::default())
        .add_systems(Update, poll_messages);

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

fn handle_server_message(
    msg: ServerMessage,
    player: &mut LocalPlayer,
    state: &mut GameState,
) {
    match msg {
        ServerMessage::JoinAck { player_id, role } => {
            player.id = Some(player_id);
            player.role = role;
            info!("JoinAck: id={player_id}, role={role:?}");
        }
        ServerMessage::GameStarted => {
            state.game_started = true;
            info!("Game started!");
        }
        ServerMessage::Event(GameEvent::StateSnapshot(sub)) => {
            state.submarine = Some(sub);
        }
        ServerMessage::Event(event) => {
            debug!("GameEvent: {event:?}");
        }
        ServerMessage::Error(e) => {
            warn!("Server error: {e:?}");
        }
    }
}
