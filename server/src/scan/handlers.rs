use crate::auth;
use crate::error::AppError;
use crate::stock::logic::settle_star_system_stock;
use crate::types::{AppState, Building, BuildingKind, Ship, StarSystemDetails, StarSystemStock};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use time::{Duration, OffsetDateTime};
use universe::{
    generator::generate_star,
    settings::{SCAN_CHARGE_RATE_LY_PER_SEC, distance_between_cells_ly},
};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct ScanRequest {
    pub target_x: i32,
    pub target_y: i32,
}

#[derive(serde::Serialize)]
pub struct ScanResponse {
    pub system_data: StarSystemDetails,
    pub cooldown_ends_at: OffsetDateTime,
}

pub async fn scan_ship_handler(
    Path(id): Path<i64>,
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
    Json(req): Json<ScanRequest>,
) -> Result<Json<ScanResponse>, AppError> {
    let empire_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid empire ID".to_string()))?;

    let ship = sqlx::query_as::<_, Ship>("SELECT * FROM ships WHERE id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    if ship.owner_id != empire_id {
        return Err(AppError::Internal("You do not own this ship".to_string()));
    }

    let now = OffsetDateTime::now_utc();
    if ship.scan_ready_at > now {
        return Err(AppError::Internal(format!(
            "Scanner is charging. Ready at {}",
            ship.scan_ready_at
        )));
    }

    let distance = distance_between_cells_ly(ship.star_x, ship.star_y, req.target_x, req.target_y);
    let cooldown_secs = (distance / SCAN_CHARGE_RATE_LY_PER_SEC).ceil() as i64;
    let cooldown_ends_at = now + Duration::new(cooldown_secs, 0);

    sqlx::query("UPDATE ships SET scan_ready_at = $1 WHERE id = $2")
        .bind(cooldown_ends_at)
        .bind(id)
        .execute(&state.pool)
        .await?;

    let system_data = fetch_star_system_details(&state.pool, req.target_x, req.target_y).await?;

    Ok(Json(ScanResponse {
        system_data,
        cooldown_ends_at,
    }))
}

pub async fn scan_building_handler(
    Path(id): Path<i64>,
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
    Json(req): Json<ScanRequest>,
) -> Result<Json<ScanResponse>, AppError> {
    let empire_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid empire ID".to_string()))?;

    let building = sqlx::query_as::<_, Building>("SELECT * FROM buildings WHERE id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    if building.kind != BuildingKind::Radar {
        return Err(AppError::Internal("Only Radars can scan".to_string()));
    }

    if building.owner_id != Some(empire_id) {
        return Err(AppError::Internal(
            "You do not own this building".to_string(),
        ));
    }

    let now = OffsetDateTime::now_utc();
    if building.scan_ready_at > now {
        return Err(AppError::Internal(format!(
            "Scanner is charging. Ready at {}",
            building.scan_ready_at
        )));
    }

    let distance =
        distance_between_cells_ly(building.star_x, building.star_y, req.target_x, req.target_y);
    let cooldown_secs = (distance / SCAN_CHARGE_RATE_LY_PER_SEC).ceil() as i64;
    let cooldown_ends_at = now + Duration::new(cooldown_secs, 0);

    sqlx::query("UPDATE buildings SET scan_ready_at = $1 WHERE id = $2")
        .bind(cooldown_ends_at)
        .bind(id)
        .execute(&state.pool)
        .await?;

    let system_data = fetch_star_system_details(&state.pool, req.target_x, req.target_y).await?;

    Ok(Json(ScanResponse {
        system_data,
        cooldown_ends_at,
    }))
}

async fn fetch_star_system_details(
    pool: &sqlx::PgPool,
    x: i32,
    y: i32,
) -> Result<StarSystemDetails, AppError> {
    let system = generate_star(x, y, Some(0))
        .ok_or_else(|| AppError::Internal("System not found".to_string()))?;

    settle_star_system_stock(pool, x, y)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let stock = sqlx::query_as::<_, StarSystemStock>(
        "SELECT star_x, star_y, last_settled_at, settled FROM star_system_stock WHERE star_x = $1 AND star_y = $2"
    )
    .bind(x)
    .bind(y)
    .fetch_optional(pool)
    .await?;

    let ships = sqlx::query_as::<_, Ship>(
        "SELECT * FROM ships WHERE star_x = $1 AND star_y = $2 AND (warp_completed_at IS NULL OR warp_completed_at <= NOW())",
    )
    .bind(x)
    .bind(y)
    .fetch_all(pool)
    .await?;

    let buildings =
        sqlx::query_as::<_, Building>("SELECT * FROM buildings WHERE star_x = $1 AND star_y = $2")
            .bind(x)
            .bind(y)
            .fetch_all(pool)
            .await?;

    Ok(StarSystemDetails {
        system,
        stock,
        buildings,
        ships,
    })
}
