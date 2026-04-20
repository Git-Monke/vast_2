use crate::auth;
use crate::types::{AppState, Ship};
use axum::{Extension, Json, extract::{Path, State}};
use serde::Deserialize;
use uuid::Uuid;

use universe::ShipAttackMode;

#[derive(Deserialize)]
pub struct SetAttackModeRequest {
    pub mode: ShipAttackMode,
}

pub async fn set_ship_attack_mode(
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<SetAttackModeRequest>,
) -> Result<(), (axum::http::StatusCode, String)> {
    let owner_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid user ID".to_string(),
        )
    })?;

    // 1. Fetch ship and verify ownership
    let _ship = sqlx::query_as::<sqlx::Postgres, Ship>(
        "SELECT * FROM ships WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "Ship not found".to_string(),
    ))?;

    // 2. Update ship attack mode
    sqlx::query("UPDATE ships SET attack_mode = $1 WHERE id = $2")
        .bind(req.mode)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(())
}
