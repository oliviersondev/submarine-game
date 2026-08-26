use axum::{Router, routing::any};
use tokio::net::TcpListener;

mod game_room;
mod lobby;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/ws", any(lobby::ws_handler));

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server listening on 0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}
