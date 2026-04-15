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
    let ship = sqlx::query_as::<_, Ship>("SELECT * FROM ships WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::Internal("Ship not found".to_string()))?;

    if ship.owner_id.to_string() != claims.sub {
        return Err(AppError::Internal("You do not own this ship".to_string()));
    }

    if ship.in_transit {
        return Err(AppError::Internal("Ship is already in transit".to_string()));
    }

    let existing_job = sqlx::query!("SELECT id FROM warp_jobs WHERE ship_id = $1", id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if existing_job.is_some() {
        return Err(AppError::Internal(
            "Ship already has a pending warp job".to_string(),
        ));
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
    let scheduled_at = OffsetDateTime::now_utc() + time::Duration::seconds_f64(duration_secs);

    let job = create_warp_job(state, id, req.x, req.y, scheduled_at).await?;

    Ok(Json(job))
}

pub async fn create_warp_job(
    state: AppState,
    ship_id: i64,
    to_star_x: i32,
    to_star_y: i32,
    scheduled_at: OffsetDateTime,
) -> Result<WarpJob, AppError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let job = sqlx::query_as::<_, WarpJob>(
        "INSERT INTO warp_jobs (scheduled_at, ship_id, to_star_x, to_star_y) 
         VALUES ($1, $2, $3, $4) 
         RETURNING id, scheduled_at, ship_id, to_star_x, to_star_y",
    )
    .bind(scheduled_at)
    .bind(ship_id)
    .bind(to_star_x)
    .bind(to_star_y)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    sqlx::query!("UPDATE ships SET in_transit = true WHERE id = $1", ship_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    spawn_warp_job_task(state, job.id, scheduled_at);

    Ok(job)
}

fn spawn_warp_job_task(state: AppState, job_id: i64, scheduled_at: OffsetDateTime) {
    let now = OffsetDateTime::now_utc();
    let delay = (scheduled_at - now).max(time::Duration::ZERO);

    tokio::spawn(async move {
        tokio::time::sleep(delay.try_into().unwrap_or_default()).await;
        if let Err(e) = complete_warp_job(state, job_id).await {
            eprintln!("Failed to complete warp job {}: {:?}", job_id, e);
        }
    });
}

async fn complete_warp_job(state: AppState, job_id: i64) -> Result<(), AppError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let job = sqlx::query!("SELECT * FROM warp_jobs WHERE id = $1", job_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::Internal("Warp job not found".to_string()))?;

    sqlx::query!(
        "UPDATE ships SET star_x = $1, star_y = $2, in_transit = false WHERE id = $3",
        job.to_star_x,
        job.to_star_y,
        job.ship_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    sqlx::query!("DELETE FROM warp_jobs WHERE id = $1", job_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(())
}

pub async fn load_warp_jobs(state: AppState) -> Result<(), AppError> {
    let jobs = sqlx::query_as::<_, WarpJob>("SELECT * FROM warp_jobs")
        .fetch_all(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    for job in jobs {
        spawn_warp_job_task(state.clone(), job.id, job.scheduled_at);
    }

    Ok(())
}
