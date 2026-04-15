use axum::{
    Extension, Json, Router, middleware,
    routing::{get, post},
};

use tracing;
use tracing_subscriber;
use uuid::Uuid;

use server::auth;
use server::jobs::warp::warp_ship_handler;
use server::types::{AppState, Ship};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = sqlx::PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to Postgres");

    let state = AppState { pool };

    let protected_routes = Router::new()
        .route("/ships", get(get_ships))
        .route("/ships/{id}/warp", post(warp_ship_handler))
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

async fn get_ships(
    Extension(claims): Extension<auth::Claims>,
    state: axum::extract::State<AppState>,
) -> Result<Json<Vec<Ship>>, (axum::http::StatusCode, String)> {
    let owner_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid user ID".to_string(),
        )
    })?;

    let ships = sqlx::query_as::<sqlx::Postgres, Ship>("SELECT * FROM ships WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ships))
}
