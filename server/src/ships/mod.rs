use crate::auth;
use crate::types::{AppState, Building, BuildingKind, Ship};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct DockRequest {
    pub star_x: i32,
    pub star_y: i32,
    pub planet_index: i16,
    pub slot_index: i16,
}

pub fn get_ship_depot_capacity_kt(level: i32) -> u32 {
    (10.0 * f32::powf(10.0, (level as f32 - 1.0) / 4.5)) as u32
}

pub async fn get_depot_used_capacity_kt(
    pool: &sqlx::PgPool,
    building_id: i64,
) -> Result<u32, sqlx::Error> {
    // Note: Later on, this must also sum the mass of ships currently under construction at this depot.
    let ships = sqlx::query_as::<sqlx::Postgres, Ship>("SELECT * FROM ships WHERE docked_at = $1")
        .bind(building_id)
        .fetch_all(pool)
        .await?;

    let used_kt: u32 = ships.iter().map(|s| s.stats.size_kt).sum();
    Ok(used_kt)
}

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

pub async fn dock_ship(
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<DockRequest>,
) -> Result<(), (axum::http::StatusCode, String)> {
    let owner_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid user ID".to_string(),
        )
    })?;

    // 1. Fetch ship
    let ship = sqlx::query_as::<sqlx::Postgres, Ship>(
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

    if ship.in_transit {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Ship is in transit".to_string(),
        ));
    }

    if ship.star_x != req.star_x || ship.star_y != req.star_y {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Ship is not in this star system".to_string(),
        ));
    }

    // 2. Fetch building
    let building = sqlx::query_as::<sqlx::Postgres, Building>(
        "SELECT * FROM buildings WHERE star_x = $1 AND star_y = $2 AND planet_index = $3 AND slot_index = $4"
    )
        .bind(req.star_x)
        .bind(req.star_y)
        .bind(req.planet_index)
        .bind(req.slot_index)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((axum::http::StatusCode::NOT_FOUND, "Building not found".to_string()))?;

    if building.kind != BuildingKind::ShipDepot {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Building is not a Ship Depot".to_string(),
        ));
    }

    // 3. Capacity checks
    let max_cap = get_ship_depot_capacity_kt(building.level);
    let used_cap = get_depot_used_capacity_kt(&state.pool, building.id)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if used_cap + ship.stats.size_kt > max_cap {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Ship depot is full".to_string(),
        ));
    }

    // 4. Update ship
    sqlx::query("UPDATE ships SET docked_at = $1 WHERE id = $2")
        .bind(building.id)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(())
}

pub async fn undock_ship(
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<(), (axum::http::StatusCode, String)> {
    let owner_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid user ID".to_string(),
        )
    })?;

    // 1. Fetch ship
    let ship = sqlx::query_as::<sqlx::Postgres, Ship>(
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

    if ship.docked_at.is_none() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Ship is not docked".to_string(),
        ));
    }

    // 2. Update ship
    sqlx::query("UPDATE ships SET docked_at = NULL WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(())
}
