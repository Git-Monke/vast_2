use crate::auth;
use crate::types::{AppState, Ship};
use axum::{Extension, Json, extract::State};
use uuid::Uuid;

pub async fn get_ships(
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
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
