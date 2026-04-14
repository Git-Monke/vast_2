mod auth;

use axum::{
    Extension, Json, Router, middleware,
    routing::{get, post},
};

use serde::Serialize;
use tracing;
use tracing_subscriber;

#[derive(Serialize, Clone)]
pub struct Ship {
    ship_name: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = sqlx::PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to Postgres");

    // Initialize tables
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create users table");

    let state = AppState { pool };

    let protected_routes =
        Router::new()
            .route("/ships", get(hello_world))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                auth::auth_middleware,
            ));

    let app = Router::new()
        .route("/register", post(auth::register_user))
        .route("/authorize", post(auth::authorize))
        .merge(protected_routes)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5280")
        .await
        .expect("Failed to bind to port");

    tracing::info!("Server is starting");

    axum::serve(listener, app).await.expect("Failed to serve");
}

async fn hello_world(Extension(claims): Extension<auth::Claims>) -> Result<Json<Ship>, String> {
    Ok(Json(Ship {
        ship_name: format!(
            "Welcome aboard, Commander {}! All systems are nominal.",
            claims.username
        ),
    }))
}
