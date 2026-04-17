use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    auth,
    presence::{check_enemy_garrison, check_presence},
    types::{AppState, Building},
};

pub async fn delete_building(
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
    Path(building_id): Path<i64>,
) -> Result<StatusCode, (StatusCode, String)> {
    let owner_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid user ID in token".to_string(),
        )
    })?;

    // Fetch building to verify ownership and location
    let building: Building = sqlx::query_as::<_, Building>("SELECT * FROM buildings WHERE id = $1")
        .bind(building_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Building not found".to_string()))?;

    // Ensure the requester owns the building
    if building.owner_id != Some(owner_id) {
        return Err((StatusCode::FORBIDDEN, "Not your building".to_string()));
    }

    // Presence and enemy garrison checks – same rules as building creation
    let is_present = check_presence(&state.pool, owner_id, building.star_x, building.star_y)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !is_present {
        return Err((
            StatusCode::FORBIDDEN,
            "Player has no presence in this star system".to_string(),
        ));
    }
    if let Some(_) = check_enemy_garrison(&state.pool, owner_id, building.star_x, building.star_y)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        return Err((
            StatusCode::FORBIDDEN,
            "Cannot delete in a system with enemy garrison".to_string(),
        ));
    }

    // Delete the building inside a transaction (future-proof if extra steps needed)
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    sqlx::query!("DELETE FROM buildings WHERE id = $1", building_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
