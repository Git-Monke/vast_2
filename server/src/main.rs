use axum::{
    Extension, Json, Router, middleware,
    extract::{Path, State},
    routing::{get, post},
};

use tracing;
use tracing_subscriber;
use uuid::Uuid;

use server::auth;
use server::jobs::warp::warp_ship_handler;
use server::types::{AppState, Ship, StarSystemStock, StarSystemDetails};
use universe::generator::generate_system;

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
        .route("/systems/{x}/{y}", get(get_star_system))
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

async fn get_star_system(
    Path((x, y)): Path<(i32, i32)>,
    Extension(_claims): Extension<auth::Claims>,
    State(state): State<AppState>,
) -> Result<Json<StarSystemDetails>, (axum::http::StatusCode, String)> {
    let system = generate_system(x, y);

    let stock = sqlx::query_as::<_, StarSystemStock>(
        "SELECT star_x, star_y, last_settled_at, capacity_kt, settled FROM star_system_stock WHERE star_x = $1 AND star_y = $2"
    )
    .bind(x)
    .bind(y)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let ships = sqlx::query_as::<_, Ship>(
        "SELECT * FROM ships WHERE star_x = $1 AND star_y = $2 AND in_transit = false"
    )
    .bind(x)
    .bind(y)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(StarSystemDetails {
        system,
        stock,
        ships,
    }))
}
