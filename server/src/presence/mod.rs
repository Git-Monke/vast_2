use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use crate::auth;
use crate::types::{AppState, Building, Ship, StarSystemDetails, StarSystemStock};
use universe::generator::generate_star;

/// Checks if a player has presence in a star system based on the player_presence table.

pub async fn check_presence(
    pool: &PgPool,
    empire_id: Uuid,
    star_x: i32,
    star_y: i32,
) -> Result<bool, AppError> {
    let exists = sqlx::query!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM player_presence 
            WHERE empire_id = $1 AND star_x = $2 AND star_y = $3
        ) as "exists!"
        "#,
        empire_id,
        star_x,
        star_y
    )
    .fetch_one(pool)
    .await?;

    Ok(exists.exists)
}

/// Updates the player_presence table for a specific player and star system.
/// Presence is granted if the player has at least one ship (not in transit)
/// or a Radar building in the system.
pub async fn update_presence(
    pool: &PgPool,
    empire_id: Uuid,
    star_x: i32,
    star_y: i32,
) -> Result<(), AppError> {
    // Check for ships
    let has_ship = sqlx::query!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM ships 
            WHERE owner_id = $1 AND star_x = $2 AND star_y = $3 AND in_transit = FALSE
        ) as "exists!"
        "#,
        empire_id,
        star_x,
        star_y
    )
    .fetch_one(pool)
    .await?
    .exists;

    // Check for Radar buildings
    let has_radar = if !has_ship {
        sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM buildings 
                WHERE owner_id = $1 AND star_x = $2 AND star_y = $3 AND kind = 'Radar'
            ) as "exists!"
            "#,
            empire_id,
            star_x,
            star_y
        )
        .fetch_one(pool)
        .await?
        .exists
    } else {
        true
    };

    if has_ship || has_radar {
        // Upsert presence
        sqlx::query!(
            r#"
            INSERT INTO player_presence (empire_id, star_x, star_y)
            VALUES ($1, $2, $3)
            ON CONFLICT (empire_id, star_x, star_y) DO NOTHING
            "#,
            empire_id,
            star_x,
            star_y
        )
        .execute(pool)
        .await?;
    } else {
        // Remove presence
        sqlx::query!(
            r#"
            DELETE FROM player_presence 
            WHERE empire_id = $1 AND star_x = $2 AND star_y = $3
            "#,
            empire_id,
            star_x,
            star_y
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn get_star_system(
    Path((x, y)): Path<(i32, i32)>,
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
) -> Result<Json<StarSystemDetails>, (axum::http::StatusCode, String)> {
    let empire_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (axum::http::StatusCode::BAD_REQUEST, "Invalid empire ID in token".to_string()))?;

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
