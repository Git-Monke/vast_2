use axum::{Extension, Json, extract::{Path, State}, http::StatusCode};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth;
use crate::types::{AppState, Building, BuildingKind};

use universe::ShipAttackMode;

#[derive(Deserialize)]
pub struct SetAttackModeRequest {
    pub mode: ShipAttackMode,
}

pub async fn set_building_attack_mode(
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<SetAttackModeRequest>,
) -> Result<(), (StatusCode, String)> {
    let owner_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid user ID".to_string(),
        )
    })?;

    // 1. Fetch building and verify ownership
    let building: Building = sqlx::query_as::<_, Building>(
        "SELECT * FROM buildings WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((
        StatusCode::NOT_FOUND,
        "Building not found".to_string(),
    ))?;

    // 2. Only MilitaryGarrison buildings support attack mode
    if building.kind != BuildingKind::MilitaryGarrison {
        return Err((
            StatusCode::FORBIDDEN,
            "Only MilitaryGarrison buildings support attack mode".to_string(),
        ));
    }

    // 3. Update building attack mode
    sqlx::query("UPDATE buildings SET attack_mode = $1 WHERE id = $2")
        .bind(req.mode)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(())
}
