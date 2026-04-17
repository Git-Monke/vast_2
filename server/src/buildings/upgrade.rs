use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth,
    presence::{check_enemy_garrison, check_presence},
    types::{AppState, Building, BuildingKind},
};

use super::prices::{get_building_cost, get_required_mass};

#[derive(Deserialize)]
pub struct UpgradeRequest {
    // For consistency we keep the same fields as the original build request.
    // Only `level` is relevant for the upgrade; other fields are validated
    // against the existing building to avoid mismatches.
    pub star_x: i32,
    pub star_y: i32,
    pub planet_index: i16,
    pub slot_index: i16,
    pub kind: BuildingKind,
    pub level: i32,
}

pub async fn upgrade_building(
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
    Path(building_id): Path<i64>,
    Json(req): Json<UpgradeRequest>,
) -> Result<Json<Building>, (StatusCode, String)> {
    let owner_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid user ID in token".to_string(),
        )
    })?;

    // Verify building exists and belongs to owner
    let existing: Building = sqlx::query_as::<_, Building>("SELECT * FROM buildings WHERE id = $1")
        .bind(building_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Building not found".to_string()))?;

    if existing.owner_id != Some(owner_id) {
        return Err((StatusCode::FORBIDDEN, "Not your building".to_string()));
    }

    // Ensure the request matches the existing building (prevent accidental changes)
    if existing.star_x != req.star_x
        || existing.star_y != req.star_y
        || existing.planet_index != req.planet_index
        || existing.slot_index != req.slot_index
        || existing.kind != req.kind
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Mismatched building data".to_string(),
        ));
    }

    if req.level <= existing.level {
        return Err((
            StatusCode::BAD_REQUEST,
            "New level must be higher".to_string(),
        ));
    }

    // Presence and enemy garrison checks (same as building construction)
    let is_present = check_presence(&state.pool, owner_id, req.star_x, req.star_y)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !is_present {
        return Err((
            StatusCode::FORBIDDEN,
            "Player has no presence in this star system".to_string(),
        ));
    }
    if let Some(_) = check_enemy_garrison(&state.pool, owner_id, req.star_x, req.star_y)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        return Err((
            StatusCode::FORBIDDEN,
            "Cannot upgrade in a system with enemy garrison".to_string(),
        ));
    }

    // Verify ship mass requirement
    let required_mass = get_required_mass(req.level);
    let max_mass: Option<f64> = sqlx::query_scalar!(
        r#"
        SELECT MAX((stats->'size_kt')::text::double precision)
        FROM ships 
        WHERE owner_id = $1 AND star_x = $2 AND star_y = $3 AND (warp_completed_at IS NULL OR warp_completed_at <= NOW())
        "#,
        owner_id,
        req.star_x,
        req.star_y
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if max_mass.unwrap_or(0.0) < required_mass {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Required ship mass {}kt not met", required_mass),
        ));
    }

    // Cost calculation – incremental cost only
    let sales_depot_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM buildings WHERE owner_id = $1 AND kind = 'SalesDepot'",
        owner_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .unwrap_or(0);

    let current_cost = get_building_cost(req.kind.clone(), existing.level, sales_depot_count);
    let new_cost = get_building_cost(req.kind.clone(), req.level, sales_depot_count);
    let incremental_cost = new_cost - current_cost;

    let user_credits: i64 =
        sqlx::query_scalar!("SELECT credits FROM users WHERE id = $1", owner_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if user_credits < incremental_cost {
        return Err((
            StatusCode::PAYMENT_REQUIRED,
            format!(
                "Insufficient credits. Need {}, have {}",
                incremental_cost, user_credits
            ),
        ));
    }

    // Transaction: deduct credits and upgrade building
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query!(
        "UPDATE users SET credits = credits - $1 WHERE id = $2",
        incremental_cost,
        owner_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // For simplicity we keep health logic as in build_building – health scales with level when applicable
    let health = if super::prices::building_has_health(&req.kind) {
        100 * req.level
    } else {
        existing.health // preserve existing health for types without health tracking
    };

    let upgraded: Building = sqlx::query_as::<_, Building>(
        r#"
        UPDATE buildings SET level = $1, health = $2 WHERE id = $3 RETURNING *
        "#,
    )
    .bind(req.level)
    .bind(health)
    .bind(building_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(upgraded))
}
