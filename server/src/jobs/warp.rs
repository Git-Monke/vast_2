use crate::auth::Claims;
use crate::error::AppError;
use crate::types::{AppState, Ship};
use axum::{Extension, Json, extract::Path, extract::State};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use universe::checker::star_is_at_point;
use universe::settings::distance_between_cells_ly;
use universe::ships::travel_duration_secs;

use time::serde::rfc3339;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WarpJob {
    pub id: i64,
    #[serde(with = "rfc3339")]
    pub scheduled_at: OffsetDateTime,
    pub ship_id: i64,
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
) -> Result<Json<WarpJob>, AppError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // 1. Check ship existence and ownership
    let ship = sqlx::query_as::<_, Ship>("SELECT * FROM ships WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::Internal("Ship not found".to_string()))?;

    if ship.owner_id.to_string() != claims.sub {
        return Err(AppError::Internal("You do not own this ship".to_string()));
    }

    // 2. Check if ship is already in transit or has a pending warp job
    if ship.in_transit {
        return Err(AppError::Internal("Ship is already in transit".to_string()));
    }

    let existing_job = sqlx::query!("SELECT id FROM warp_jobs WHERE ship_id = $1", id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if existing_job.is_some() {
        return Err(AppError::Internal(
            "Ship already has a pending warp job".to_string(),
        ));
    }

    // 3. Check jump readiness
    if ship.jump_ready_at > OffsetDateTime::now_utc() {
        return Err(AppError::Internal(
            "Ship jump drive is recharging".to_string(),
        ));
    }

    // 4. Validate target star existence
    if !star_is_at_point(req.x, req.y) {
        return Err(AppError::Internal(
            "No star exists at target coordinates".to_string(),
        ));
    }

    // 5. Calculate distance and check range
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

    // 6. Calculate arrival time
    let duration_secs = travel_duration_secs(distance, ship.stats.speed_lys);
    let scheduled_at = OffsetDateTime::now_utc() + time::Duration::seconds_f64(duration_secs);

    // 7. Create warp job and update ship status
    let job = sqlx::query_as::<_, WarpJob>(
        "INSERT INTO warp_jobs (scheduled_at, ship_id, to_star_x, to_star_y) 
         VALUES ($1, $2, $3, $4) 
         RETURNING id, scheduled_at, ship_id, to_star_x, to_star_y",
    )
    .bind(scheduled_at)
    .bind(id)
    .bind(req.x)
    .bind(req.y)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    sqlx::query!("UPDATE ships SET in_transit = true WHERE id = $1", id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(job))
}
