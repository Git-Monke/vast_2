use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::types::Json as SqlxJson;
use universe::{Material, credits_for_materials_sale};
use uuid::Uuid;

use crate::{
    auth,
    types::{AppState, Ship},
};

#[derive(serde::Serialize)]
pub struct SellResult {
    earned: u64,
    remaining_cargo: Vec<Material>,
}

pub async fn sell_ship_cargo(
    Extension(claims): Extension<auth::Claims>,
    Path(ship_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<SellResult>, (StatusCode, String)> {
    // Verify user
    let owner_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid user ID in token".to_string(),
        )
    })?;

    // Fetch ship with a lock (SELECT ... FOR UPDATE) inside a transaction
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let ship: Ship = sqlx::query_as::<_, Ship>("SELECT * FROM ships WHERE id = $1 FOR UPDATE")
        .bind(ship_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    // Ownership check
    if ship.owner_id != owner_id {
        return Err((
            StatusCode::FORBIDDEN,
            "Ship does not belong to you".to_string(),
        ));
    }

    // Check for owned SalesDepot in the same system
    let depot_exists: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM buildings WHERE star_x = $1 AND star_y = $2 AND kind = 'SalesDepot' AND owner_id = $3)",
        ship.star_x,
        ship.star_y,
        owner_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .unwrap_or(false);

    if !depot_exists {
        return Err((
            StatusCode::FORBIDDEN,
            "No owned SalesDepot in this system".to_string(),
        ));
    }

    // Calculate credits from cargo
    let cargo: Vec<Material> = ship.cargo.0.clone(); // unwrap Json
    let earned = credits_for_materials_sale(&cargo);

    // Update user credits and clear ship cargo
    sqlx::query("UPDATE users SET credits = credits + $1 WHERE id = $2")
        .bind(earned as i64)
        .bind(owner_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query("UPDATE ships SET cargo = $1 WHERE id = $2")
        .bind(SqlxJson::<Vec<Material>>::from(Vec::new()))
        .bind(ship_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SellResult {
        earned,
        remaining_cargo: Vec::new(),
    }))
}
