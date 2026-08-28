use std::{collections::HashMap, sync::Arc, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use shared::{
    codec::{decode, encode},
    ClientMessage, ClientPayload, CrewRole, LobbyCommand, LobbyPhase, LobbySnapshot,
    MissionCommand, MissionConfig, PlayerId, ProtocolError, RoleOccupant, RoleSlot, RoomId,
    ServerMessage, ServerPayload, SessionId, PROTOCOL_VERSION,
};
use tokio::sync::{mpsc, RwLock};

use crate::game_room::{self, GameRoomAction, GameRoomCommand};

struct PlayerEntry {
    role: CrewRole,
    ready: bool,
    tx: mpsc::Sender<ServerMessage>,
}

struct Room {
    config: MissionConfig,
    phase: LobbyPhase,
    players: HashMap<PlayerId, PlayerEntry>,
    game_cmd_tx: Option<mpsc::Sender<GameRoomCommand>>,
}

pub struct RoomRegistry {
    rooms: HashMap<RoomId, Room>,
    next_room: u64,
    next_session: u64,
    next_player: u64,
}

impl RoomRegistry {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            next_room: 1,
            next_session: 1,
            next_player: 1,
        }
    }

    fn create_room(
        &mut self,
        role: CrewRole,
        tx: mpsc::Sender<ServerMessage>,
    ) -> (RoomId, SessionId, PlayerId) {
        let room_number = self.next_room;
        self.next_room += 1;
        let room_id = RoomId(format!("{room_number:06}"));
        self.rooms.insert(
            room_id.clone(),
            Room {
                config: MissionConfig::new(room_number),
                phase: LobbyPhase::Waiting,
                players: HashMap::new(),
                game_cmd_tx: None,
            },
        );
        let (session_id, player_id) = self.allocate_identity();
        self.rooms.get_mut(&room_id).unwrap().players.insert(
            player_id,
            PlayerEntry {
                role,
                ready: false,
                tx,
            },
        );
        (room_id, session_id, player_id)
    }

    fn join_room(
        &mut self,
        room_id: &RoomId,
        role: CrewRole,
        tx: mpsc::Sender<ServerMessage>,
    ) -> Result<(SessionId, PlayerId), ProtocolError> {
        if !valid_room_code(&room_id.0) {
            return Err(ProtocolError::InvalidRoomCode);
        }
        let room = self.rooms.get(room_id).ok_or(ProtocolError::RoomNotFound)?;
        if room.phase != LobbyPhase::Waiting {
            return Err(ProtocolError::RoomAlreadyStarted);
        }
        if room.players.values().any(|player| player.role == role) {
            return Err(ProtocolError::RoleAlreadyTaken(role));
        }
        let (session_id, player_id) = self.allocate_identity();
        self.rooms.get_mut(room_id).unwrap().players.insert(
            player_id,
            PlayerEntry {
                role,
                ready: false,
                tx,
            },
        );
        Ok((session_id, player_id))
    }

    fn allocate_identity(&mut self) -> (SessionId, PlayerId) {
        let session = SessionId(self.next_session);
        let player = PlayerId(self.next_player);
        self.next_session += 1;
        self.next_player += 1;
        (session, player)
    }

    fn snapshot(&self, room_id: &RoomId) -> Option<LobbySnapshot> {
        let room = self.rooms.get(room_id)?;
        Some(LobbySnapshot {
            room_id: room_id.clone(),
            phase: room.phase,
            slots: CrewRole::ALL
                .into_iter()
                .map(|role| RoleSlot {
                    role,
                    occupant: room
                        .players
                        .iter()
                        .find(|(_, player)| player.role == role)
                        .map(|(player_id, player)| RoleOccupant::Human {
                            player_id: *player_id,
                            ready: player.ready,
                        })
                        .unwrap_or(RoleOccupant::Bot),
                })
                .collect(),
        })
    }

    fn broadcast_lobby(&self, room_id: &RoomId) {
        let Some(room) = self.rooms.get(room_id) else {
            return;
        };
        let Some(snapshot) = self.snapshot(room_id) else {
            return;
        };
        let message = ServerMessage::new(ServerPayload::Lobby(snapshot));
        for player in room.players.values() {
            let _ = player.tx.try_send(message.clone());
        }
    }

    fn set_ready(&mut self, room_id: &RoomId, player_id: PlayerId, ready: bool) {
        if let Some(player) = self
            .rooms
            .get_mut(room_id)
            .and_then(|room| room.players.get_mut(&player_id))
        {
            player.ready = ready;
        }
    }

    fn start_room(&mut self, room_id: &RoomId) -> Result<(), ProtocolError> {
        let room = self
            .rooms
            .get_mut(room_id)
            .ok_or(ProtocolError::RoomNotFound)?;
        if room.phase != LobbyPhase::Waiting {
            return Err(ProtocolError::RoomAlreadyStarted);
        }
        room.phase = LobbyPhase::Running;
        let config = room.config;
        let players: HashMap<_, _> = room
            .players
            .iter()
            .map(|(id, player)| (*id, (player.role, player.tx.clone())))
            .collect();
        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        room.game_cmd_tx = Some(cmd_tx);
        for player in room.players.values() {
            let _ = player
                .tx
                .try_send(ServerMessage::new(ServerPayload::MissionStarted { config }));
        }
        tokio::spawn(game_room::run(config, cmd_rx, players));
        Ok(())
    }

    fn remove(&mut self, room_id: &RoomId, player_id: PlayerId) {
        let remove_room = if let Some(room) = self.rooms.get_mut(room_id) {
            if room.phase == LobbyPhase::Waiting {
                room.players.remove(&player_id);
            }
            room.players.is_empty()
        } else {
            false
        };
        if remove_room {
            self.rooms.remove(room_id);
        }
    }
}

fn valid_room_code(code: &str) -> bool {
    code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit())
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(registry): State<Arc<RwLock<RoomRegistry>>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, registry))
}

async fn handle_socket(mut socket: WebSocket, registry: Arc<RwLock<RoomRegistry>>) {
    let Some(message) = receive_client_message(&mut socket).await else {
        return;
    };
    if message.version != PROTOCOL_VERSION {
        send_error_and_close(
            &mut socket,
            ProtocolError::IncompatibleVersion {
                expected: PROTOCOL_VERSION,
                received: message.version,
            },
        )
        .await;
        return;
    }

    let (tx, mut rx) = mpsc::channel(32);
    let registration = {
        let mut registry = registry.write().await;
        match message.payload {
            ClientPayload::Lobby(LobbyCommand::CreateRoom { role }) => {
                let (room_id, session_id, player_id) = registry.create_room(role, tx);
                Ok((room_id, session_id, player_id, role))
            }
            ClientPayload::Lobby(LobbyCommand::JoinRoom { room_id, role }) => registry
                .join_room(&room_id, role, tx)
                .map(|(session_id, player_id)| (room_id, session_id, player_id, role)),
            _ => Err(ProtocolError::RoomNotFound),
        }
    };
    let (room_id, session_id, player_id, role) = match registration {
        Ok(value) => value,
        Err(error) => {
            send_error_and_close(&mut socket, error).await;
            return;
        }
    };

    let joined = ServerMessage::new(ServerPayload::SessionJoined {
        session_id,
        player_id,
        room_id: room_id.clone(),
        role,
    });
    if send_message(&mut socket, &joined).await.is_err() {
        registry.write().await.remove(&room_id, player_id);
        return;
    }
    {
        let registry = registry.read().await;
        registry.broadcast_lobby(&room_id);
    }

    loop {
        tokio::select! {
            Some(message) = rx.recv() => {
                if send_message(&mut socket, &message).await.is_err() {
                    break;
                }
            }
            incoming = socket.recv() => {
                let Some(Ok(Message::Binary(bytes))) = incoming else { break };
                let Ok(message) = decode::<ClientMessage>(bytes.as_ref()) else { continue };
                if message.version != PROTOCOL_VERSION { continue; }
                handle_client_message(&registry, &room_id, player_id, role, message.payload).await;
            }
        }
    }

    let mut registry = registry.write().await;
    registry.remove(&room_id, player_id);
    registry.broadcast_lobby(&room_id);
}

async fn handle_client_message(
    registry: &Arc<RwLock<RoomRegistry>>,
    room_id: &RoomId,
    player_id: PlayerId,
    role: CrewRole,
    payload: ClientPayload,
) {
    match payload {
        ClientPayload::Lobby(LobbyCommand::SetReady { ready }) => {
            let mut registry = registry.write().await;
            registry.set_ready(room_id, player_id, ready);
            registry.broadcast_lobby(room_id);
        }
        ClientPayload::Lobby(LobbyCommand::StartMission) => {
            let mut registry = registry.write().await;
            if let Err(error) = registry.start_room(room_id) {
                registry.send_to_player(room_id, player_id, None, error);
            } else {
                registry.broadcast_lobby(room_id);
            }
        }
        ClientPayload::Mission(command) => {
            let (command_id, action) = match command {
                MissionCommand::Player {
                    command_id,
                    command,
                } => (command_id, GameRoomAction::Player(command)),
                MissionCommand::OrderPilotBot { command_id, order } => {
                    (command_id, GameRoomAction::OrderPilotBot(order))
                }
            };
            let sender = registry
                .read()
                .await
                .rooms
                .get(room_id)
                .and_then(|room| room.game_cmd_tx.clone());
            if let Some(sender) = sender {
                let _ = sender
                    .send(GameRoomCommand {
                        player_id,
                        role,
                        command_id,
                        action,
                    })
                    .await;
            } else {
                registry.write().await.send_to_player(
                    room_id,
                    player_id,
                    Some(command_id),
                    ProtocolError::GameNotStarted,
                );
            }
        }
        _ => {}
    }
}

impl RoomRegistry {
    fn send_to_player(
        &self,
        room_id: &RoomId,
        player_id: PlayerId,
        command_id: Option<shared::CommandId>,
        error: ProtocolError,
    ) {
        if let Some(player) = self
            .rooms
            .get(room_id)
            .and_then(|room| room.players.get(&player_id))
        {
            let _ = player.tx.try_send(ServerMessage::new(ServerPayload::Error {
                command_id,
                error,
            }));
        }
    }
}

async fn receive_client_message(socket: &mut WebSocket) -> Option<ClientMessage> {
    loop {
        match socket.recv().await {
            Some(Ok(Message::Binary(bytes))) => {
                if let Ok(message) = decode(bytes.as_ref()) {
                    return Some(message);
                }
            }
            Some(Ok(_)) => {}
            _ => return None,
        }
    }
}

async fn send_message(socket: &mut WebSocket, message: &ServerMessage) -> Result<(), axum::Error> {
    socket.send(Message::Binary(encode(message).into())).await
}

async fn send_error_and_close(socket: &mut WebSocket, error: ProtocolError) {
    if send_message(
        socket,
        &ServerMessage::new(ServerPayload::Error {
            command_id: None,
            error,
        }),
    )
    .await
    .is_err()
    {
        return;
    }

    let peer_closed = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            match socket.recv().await {
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                Some(Ok(_)) => {}
            }
        }
    })
    .await
    .is_ok();

    if !peer_closed {
        let _ = socket.send(Message::Close(None)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel() -> mpsc::Sender<ServerMessage> {
        mpsc::channel(8).0
    }

    #[test]
    fn roles_are_unique_within_a_room() {
        let mut registry = RoomRegistry::new();
        let (room_id, _, _) = registry.create_room(CrewRole::Captain, channel());

        assert!(matches!(
            registry.join_room(&room_id, CrewRole::Captain, channel()),
            Err(ProtocolError::RoleAlreadyTaken(CrewRole::Captain))
        ));
        assert!(registry
            .join_room(&room_id, CrewRole::Pilot, channel())
            .is_ok());
    }

    #[test]
    fn rejects_malformed_room_codes_before_lookup() {
        let mut registry = RoomRegistry::new();

        for code in ["11", "00000A", "0000001"] {
            assert_eq!(
                registry.join_room(&RoomId(code.to_owned()), CrewRole::Pilot, channel()),
                Err(ProtocolError::InvalidRoomCode)
            );
        }
    }

    #[tokio::test]
    async fn solo_start_fills_four_roles_with_bots() {
        let mut registry = RoomRegistry::new();
        let (room_id, _, _) = registry.create_room(CrewRole::Captain, channel());

        registry.start_room(&room_id).unwrap();
        let snapshot = registry.snapshot(&room_id).unwrap();

        assert_eq!(snapshot.phase, LobbyPhase::Running);
        assert_eq!(
            snapshot
                .slots
                .iter()
                .filter(|slot| matches!(slot.occupant, RoleOccupant::Bot))
                .count(),
            4
        );
    }

    #[test]
    fn rooms_keep_independent_members_and_seeds() {
        let mut registry = RoomRegistry::new();
        let (first, _, _) = registry.create_room(CrewRole::Captain, channel());
        let (second, _, _) = registry.create_room(CrewRole::Captain, channel());
        registry
            .join_room(&first, CrewRole::Pilot, channel())
            .unwrap();

        assert_eq!(registry.rooms[&first].players.len(), 2);
        assert_eq!(registry.rooms[&second].players.len(), 1);
        assert_ne!(
            registry.rooms[&first].config,
            registry.rooms[&second].config
        );
    }

    #[test]
    fn session_and_player_ids_are_distinct_opaque_values() {
        let mut registry = RoomRegistry::new();
        let (room_id, first_session, first_player) =
            registry.create_room(CrewRole::Captain, channel());
        let (second_session, second_player) = registry
            .join_room(&room_id, CrewRole::Pilot, channel())
            .unwrap();

        assert_ne!(first_session, second_session);
        assert_ne!(first_player, second_player);
        assert_eq!(registry.rooms[&room_id].players.len(), 2);
    }
}
