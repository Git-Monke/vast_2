use axum::{Json, Router, routing::get};

use serde::Serialize;
use tracing;
use tracing_subscriber;

#[derive(Serialize, Clone)]
pub struct Ship {
    ship_name: String,
}

#[derive(Clone)]
pub struct AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState {};

    let app = Router::new()
        .route("/ships", get(hello_world))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5280")
        .await
        .expect("Failed to bind to port");

    tracing::info!("Server is starting");

    axum::serve(listener, app).await.expect("Failed to serve");
}

async fn hello_world() -> Result<Json<Ship>, String> {
    Ok(Json(Ship {
        ship_name: "Hello!".to_owned(),
    }))
}
