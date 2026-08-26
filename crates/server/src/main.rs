use std::sync::Arc;

use axum::{routing::any, Router};
use tokio::{net::TcpListener, sync::RwLock};

mod game_room;
mod lobby;

use lobby::LobbyState;

#[tokio::main]
async fn main() {
    let state = Arc::new(RwLock::new(LobbyState::new()));

    let app = Router::new()
        .route("/ws", any(lobby::ws_handler))
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server listening on 0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}
