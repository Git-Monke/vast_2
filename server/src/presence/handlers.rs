use crate::auth;
use crate::presence::logic::check_presence;
use crate::types::{AppState, Building, Ship, StarSystemDetails, StarSystemStock};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use universe::generator::generate_star;
use uuid::Uuid;

pub async fn get_star_system(
    Path((x, y)): Path<(i32, i32)>,
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
) -> Result<Json<StarSystemDetails>, (axum::http::StatusCode, String)> {
    let empire_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid empire ID in token".to_string(),
        )
    })?;

    let has_presence = check_presence(&state.pool, empire_id, x, y)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !has_presence {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            "You do not have presence in this system".to_string(),
        ));
    }

    let system = generate_star(x, y, Some(0)).ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "System not found".to_string(),
    ))?;

    let stock = sqlx::query_as::<_, StarSystemStock>(
        "SELECT star_x, star_y, last_settled_at, capacity_kt, settled FROM star_system_stock WHERE star_x = $1 AND star_y = $2"
    )
    .bind(x)
    .bind(y)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let ships = sqlx::query_as::<_, Ship>(
        "SELECT * FROM ships WHERE star_x = $1 AND star_y = $2 AND in_transit = false",
    )
    .bind(x)
    .bind(y)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let buildings = sqlx::query_as::<_, Building>(
        "SELECT id, star_x, star_y, planet_index, slot_index, kind, level, degradation_percent, mining_material, owner_id, attack_mode, health FROM buildings WHERE star_x = $1 AND star_y = $2"
    )
    .bind(x)
    .bind(y)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(StarSystemDetails {
        system,
        stock,
        buildings,
        ships,
    }))
}
