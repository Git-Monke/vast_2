pub mod warp;

use crate::battle::logic::execute_battle;
use crate::error::AppError;
use crate::presence::{check_enemy_strike_first, update_presence};
use crate::types::AppState;
use crate::types::Ship;
use time::{Duration, OffsetDateTime};
use tokio::time::sleep;
use universe::ShipAttackMode;

/// Spawn a background task that waits until the ship's warp completion time
/// and then calls `handle_ship_arrival`.
pub fn spawn_arrival_task(
    state: AppState,
    ship_id: i64,
    warp_completed_at: OffsetDateTime,
    to_x: i32,
    to_y: i32,
) {
    let now = OffsetDateTime::now_utc();
    let delay = (warp_completed_at - now).max(Duration::ZERO);
    tokio::spawn(async move {
        let dur = delay.try_into().unwrap_or_default();
        sleep(dur).await;
        if let Err(e) = handle_ship_arrival(state, ship_id, to_x, to_y).await {
            eprintln!("Failed to complete warp for ship {}: {:?}", ship_id, e);
        }
    });
}

/// Core logic that runs when a ship actually arrives.
/// Currently this only updates presence; battle logic can be added later.
pub async fn handle_ship_arrival(
    state: AppState,
    ship_id: i64,
    to_x: i32,
    to_y: i32,
) -> Result<(), AppError> {
    let ship = sqlx::query_as::<_, Ship>("SELECT * FROM ships WHERE id = $1")
        .bind(ship_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // update presence for the arriving player
    update_presence(&state.pool, ship.owner_id, to_x, to_y)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Determine who should be the battle initiator.
    let initiator_opt = if ship.attack_mode == ShipAttackMode::StrikeFirst {
        // Arriving ship is aggressive – it initiates the fight.
        Some(ship.owner_id)
    } else {
        // Arriving ship is defensive – check if any enemy in the system has StrikeFirst.
        let enemy_has_sf = check_enemy_strike_first(&state.pool, to_x, to_y, ship.owner_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if enemy_has_sf {
            // Enemy is aggressive, but the rule says the arriving ship becomes initiator.
            Some(ship.owner_id)
        } else {
            None
        }
    };

    if let Some(initiator) = initiator_opt {
        // Run the battle using the shared battle logic.
        execute_battle(&state.pool, to_x, to_y, initiator)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

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
            row.star_x,
            row.star_y,
        );
    }
    Ok(())
}
