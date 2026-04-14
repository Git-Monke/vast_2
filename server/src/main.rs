mod auth;

use axum::{
    Extension, Json, Router, middleware,
    routing::{get, post},
};

use serde::{Deserialize, Serialize};
use tracing;
use tracing_subscriber;
use universe::{Material, ShipAttackMode, ShipStats};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct Ship {
    pub id: i64,
    pub owner_id: Uuid,
    pub stats: JsonValue<ShipStats>,
    pub cargo: JsonValue<Vec<Material>>,
    pub attack_mode: String,
    pub in_transit: bool,
    pub star_x: i32,
    pub star_y: i32,
    pub jump_ready_at: time::OffsetDateTime,
    pub health: i32,
    pub docked_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct JsonValue<T>(pub T);

impl<T> sqlx::Type<sqlx::Postgres> for JsonValue<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <serde_json::Value as sqlx::Type<sqlx::Postgres>>::type_info()
    }
}

impl<'r, T> sqlx::Decode<'r, sqlx::Postgres> for JsonValue<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let json = <serde_json::Value as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        serde_json::from_value(json).map(JsonValue).map_err(Into::into)
    }
}

impl<'a, T> sqlx::Encode<'a, sqlx::Postgres> for JsonValue<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let json = serde_json::to_value(&self.0)?;
        <serde_json::Value as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&json, buf)
    }
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

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS ships (
            id BIGSERIAL PRIMARY KEY,
            owner_id UUID NOT NULL REFERENCES users(id),
            stats JSONB NOT NULL,
            cargo JSONB NOT NULL,
            attack_mode TEXT NOT NULL,
            in_transit BOOLEAN NOT NULL DEFAULT FALSE,
            star_x INTEGER NOT NULL,
            star_y INTEGER NOT NULL,
            jump_ready_at TIMESTAMPTZ NOT NULL,
            health INTEGER NOT NULL,
            docked_at BIGINT,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create ships table");

    let state = AppState { pool };

    let protected_routes = Router::new()
        .route("/ships", get(get_ships))
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

    let ships = sqlx::query_as::<_, Ship>("SELECT * FROM ships WHERE owner_id = $1")
        .bind(owner_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ships))
}
