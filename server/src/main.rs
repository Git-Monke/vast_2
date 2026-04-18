use axum::{
    Router, middleware,
    routing::{delete, get, post},
};

use tracing;
use tracing_subscriber;

use server::auth;
use server::battle::handlers::battle_handler;
use server::buildings;
use server::jobs;
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
        .route("/ships/{id}/sell", post(ships::sell_ship_cargo))
        .route("/ships/{id}/collect", post(ships::collect_from_stock))
        .route("/systems/{x}/{y}", get(presence::get_star_system))
        .route("/systems/{x}/{y}/battle", post(battle_handler))
        .route("/buildings", post(buildings::build_building))
        .route("/buildings/{id}/upgrade", post(buildings::upgrade_building))
        .route("/buildings/{id}", delete(buildings::delete_building))
        .route(
            "/scan/ship/{id}",
            post(server::scan::handlers::scan_ship_handler),
        )
        .route(
            "/scan/building/{id}",
            post(server::scan::handlers::scan_building_handler),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ));

    let app = Router::new()
        .route("/register", post(auth::register_user))
        .route("/authenticate", post(auth::authorize))
        .merge(protected_routes)
        .with_state(state.clone());

    // Load pending warp arrival tasks for ships that are still in transit.
    // Errors are logged but do not stop server startup.
    if let Err(e) = jobs::load_arrival_tasks(state.clone()).await {
        eprintln!("Failed to load arrival tasks: {:?}", e);
    }

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5280")
        .await
        .expect("Failed to bind to port");

    tracing::info!("Server is starting");

    axum::serve(listener, app).await.expect("Failed to serve");
}
