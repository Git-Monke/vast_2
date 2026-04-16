use axum::{
    Router, middleware,
    routing::{get, post},
};

use tracing;
use tracing_subscriber;

use server::auth;
use server::buildings;
use server::jobs::warp::warp_ship_handler;
use server::presence;
use server::ships;
use server::types::AppState;

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
        .route("/ships", get(ships::get_ships))
        .route("/ships/{id}/warp", post(warp_ship_handler))
        .route("/ships/{id}/dock", post(ships::dock_ship))
        .route("/ships/{id}/undock", post(ships::undock_ship))
        .route("/systems/{x}/{y}", get(presence::get_star_system))
        .route("/buildings", post(buildings::build_building))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ));

    let app = Router::new()
        .route("/register", post(auth::register_user))
        .route("/authenticate", post(auth::authorize))
        .merge(protected_routes)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5280")
        .await
        .expect("Failed to bind to port");

    tracing::info!("Server is starting");

    axum::serve(listener, app).await.expect("Failed to serve");
}
