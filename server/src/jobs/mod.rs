pub mod warp;

use crate::error::AppError;
use crate::presence::update_presence;
use crate::types::AppState;
use time::{Duration, OffsetDateTime};
use tokio::time::sleep;

/// Spawn a background task that waits until the ship's warp completion time
/// and then calls `handle_ship_arrival`.
pub fn spawn_arrival_task(
    state: AppState,
    ship_id: i64,
    warp_completed_at: OffsetDateTime,
    from_x: i32,
    from_y: i32,
    _to_x: i32,
    _to_y: i32,
) {
    let now = OffsetDateTime::now_utc();
    let delay = (warp_completed_at - now).max(Duration::ZERO);
    tokio::spawn(async move {
        let dur = delay.try_into().unwrap_or_default();
        sleep(dur).await;
        if let Err(e) = handle_ship_arrival(state, ship_id, from_x, from_y, _to_x, _to_y).await {
            eprintln!("Failed to complete warp for ship {}: {:?}", ship_id, e);
        }
    });
}

/// Core logic that runs when a ship actually arrives.
/// Currently this only updates presence; battle logic can be added later.
pub async fn handle_ship_arrival(
    state: AppState,
    ship_id: i64,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
) -> Result<(), AppError> {
    let owner = sqlx::query!("SELECT owner_id FROM ships WHERE id = $1", ship_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    update_presence(&state.pool, owner.owner_id, from_x, from_y)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // put battle execution logic here

    Ok(())
}

/// On server startup, look for ships that have a future `warp_completed_at`
/// and schedule arrival tasks for them.
pub async fn load_arrival_tasks(state: AppState) -> Result<(), AppError> {
    let now = OffsetDateTime::now_utc();
    let rows = sqlx::query!(
        "SELECT id, warp_completed_at, star_x, star_y, from_star_x, from_star_y FROM ships WHERE warp_completed_at > $1",
        now
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    for row in rows {
        spawn_arrival_task(
            state.clone(),
            row.id,
            row.warp_completed_at.unwrap(),
            row.from_star_x.unwrap(),
            row.from_star_y.unwrap(),
            row.star_x,
            row.star_y,
        );
    }
    Ok(())
}
