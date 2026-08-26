use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use shared::{
    codec::{decode, encode},
    ClientMessage, CrewRole, PlayerCommand, ProtocolError, ServerMessage,
};
use tokio::sync::{mpsc, RwLock};

use crate::game_room;

struct PlayerEntry {
    role: CrewRole,
    tx: mpsc::Sender<ServerMessage>,
}

pub struct LobbyState {
    players: HashMap<u32, PlayerEntry>,
    next_id: u32,
    game_cmd_tx: Option<mpsc::Sender<(CrewRole, PlayerCommand)>>,
}

impl LobbyState {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
            next_id: 1,
            game_cmd_tx: None,
        }
    }

    fn role_taken(&self, role: CrewRole) -> bool {
        self.players.values().any(|p| p.role == role)
    }

    fn register(&mut self, role: CrewRole, tx: mpsc::Sender<ServerMessage>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.players.insert(id, PlayerEntry { role, tx });
        id
    }

    fn all_roles_filled(&self) -> bool {
        use CrewRole::*;
        [Captain, Pilot, Sonar, Engineer, Weapons]
            .iter()
            .all(|r| self.players.values().any(|p| &p.role == r))
    }

    fn broadcast(&self, msg: &ServerMessage) {
        for entry in self.players.values() {
            let _ = entry.tx.try_send(msg.clone());
        }
    }

    fn remove(&mut self, id: u32) {
        self.players.remove(&id);
    }

    fn start_game(&mut self) {
        let (cmd_tx, cmd_rx) = mpsc::channel::<(CrewRole, PlayerCommand)>(64);
        let player_txs: HashMap<u32, (CrewRole, mpsc::Sender<ServerMessage>)> = self
            .players
            .iter()
            .map(|(id, e)| (*id, (e.role, e.tx.clone())))
            .collect();
        self.game_cmd_tx = Some(cmd_tx);
        tokio::spawn(game_room::run(cmd_rx, player_txs));
    }

    fn game_cmd_tx(&self) -> Option<mpsc::Sender<(CrewRole, PlayerCommand)>> {
        self.game_cmd_tx.clone()
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(lobby): State<Arc<RwLock<LobbyState>>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, lobby))
}

async fn handle_socket(mut socket: WebSocket, lobby: Arc<RwLock<LobbyState>>) {
    // Wait for JoinLobby
    let role = loop {
        match socket.recv().await {
            Some(Ok(Message::Binary(b))) => match decode::<ClientMessage>(b.as_ref()) {
                Ok(ClientMessage::JoinLobby { role }) => break role,
                _ => continue,
            },
            _ => return,
        }
    };

    // Register player or reject
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(32);
    let player_id = {
        let mut state = lobby.write().await;
        if state.role_taken(role) {
            let msg = encode(&ServerMessage::Error(ProtocolError::RoleAlreadyTaken(role)));
            let _ = socket.send(Message::Binary(msg.into())).await;
            return;
        }
        let id = state.register(role, tx);
        if state.all_roles_filled() {
            state.broadcast(&ServerMessage::GameStarted);
            state.start_game();
        }
        id
    };

    // Send JoinAck
    let ack = encode(&ServerMessage::JoinAck { player_id, role });
    if socket.send(Message::Binary(ack.into())).await.is_err() {
        lobby.write().await.remove(player_id);
        return;
    }

    // Bidirectional relay loop
    loop {
        tokio::select! {
            Some(msg) = rx.recv() => {
                let bytes = encode(&msg);
                if socket.send(Message::Binary(bytes.into())).await.is_err() {
                    break;
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Binary(b))) => {
                        if let Ok(ClientMessage::Command(cmd)) = decode::<ClientMessage>(b.as_ref()) {
                            match lobby.read().await.game_cmd_tx() {
                                Some(cmd_tx) => { let _ = cmd_tx.send((role, cmd)).await; }
                                None => {
                                    let err = encode(&ServerMessage::Error(ProtocolError::GameNotStarted));
                                    if socket.send(Message::Binary(err.into())).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    lobby.write().await.remove(player_id);
}
