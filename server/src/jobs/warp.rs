use crate::auth::Claims;
use crate::error::AppError;

use crate::presence::update_presence;
use crate::types::{AppState, Ship};
use axum::{Extension, Json, extract::Path, extract::State};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use universe::checker::star_is_at_point;
use universe::settings::distance_between_cells_ly;
use universe::ships::travel_duration_secs;

#[derive(Debug, Serialize, Deserialize)]
pub struct WarpResponse {
    pub ship_id: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub warp_completed_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub jump_ready_at: OffsetDateTime,
    pub to_star_x: i32,
    pub to_star_y: i32,
}

#[derive(Deserialize)]
pub struct WarpRequest {
    pub x: i32,
    pub y: i32,
}

pub async fn warp_ship_handler(
    Path(id): Path<i64>,
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(req): Json<WarpRequest>,
) -> Result<Json<WarpResponse>, AppError> {
    let ship = sqlx::query_as::<_, Ship>("SELECT * FROM ships WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::Internal("Ship not found".to_string()))?;

    if ship.owner_id.to_string() != claims.sub {
        return Err(AppError::Internal("You do not own this ship".to_string()));
    }

    if ship.is_warping() {
        return Err(AppError::Internal("Ship is already warping".to_string()));
    }

    if ship.docked_at.is_some() {
        return Err(AppError::Internal("Ship is docked".to_string()));
    }

    if ship.jump_ready_at > OffsetDateTime::now_utc() {
        return Err(AppError::Internal(
            "Ship jump drive is recharging".to_string(),
        ));
    }

    if !star_is_at_point(req.x, req.y) {
        return Err(AppError::Internal(
            "No star exists at target coordinates".to_string(),
        ));
    }

    let distance = distance_between_cells_ly(ship.star_x, ship.star_y, req.x, req.y);
    if distance > ship.stats.battery_ly as f64 {
        return Err(AppError::Internal(format!(
            "Target out of range. Distance: {:.2} ly, Range: {} ly",
            distance, ship.stats.battery_ly
        )));
    }

    if distance <= 0.0 {
        return Err(AppError::Internal("Ship is already at target".to_string()));
    }

    let duration_secs = travel_duration_secs(distance, ship.stats.speed_lys);
    let warp_completed_at = OffsetDateTime::now_utc() + time::Duration::seconds_f64(duration_secs);

    let (star_type, _) = universe::generator::star_info_at(req.x, req.y)
        .ok_or_else(|| AppError::Internal("Target star not found".to_string()))?;

    let recharge_secs = universe::ships::battery_charge_duration_secs(
        ship.stats.size_kt,
        ship.stats.battery_ly,
        star_type.temperature_k(),
    );
    let jump_ready_at = warp_completed_at + time::Duration::seconds_f64(recharge_secs);

    let from_star_x = ship.star_x;
    let from_star_y = ship.star_y;

    // Update ship row with new coordinates and timers
    sqlx::query!(
        "UPDATE ships SET star_x = $1, star_y = $2, warp_completed_at = $3, jump_ready_at = $4, from_star_x = $5, from_star_y = $6 WHERE id = $7",
        req.x,
        req.y,
        warp_completed_at,
        jump_ready_at,
        from_star_x,
        from_star_y,
        id
    )
    .execute(&state.pool)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    crate::jobs::spawn_arrival_task(state.clone(), id, warp_completed_at, req.x, req.y);

    // Update player presence in case that was the last ship in the system
    update_presence(&state.pool, ship.owner_id, from_star_x, from_star_y)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(WarpResponse {
        ship_id: id,
        warp_completed_at,
        jump_ready_at,
        to_star_x: req.x,
        to_star_y: req.y,
    }))
}
