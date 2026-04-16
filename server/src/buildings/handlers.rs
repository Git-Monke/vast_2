use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth,
    presence::check_presence,
    types::{AppState, Building, BuildingKind},
};

use super::prices::{
    building_has_health, building_has_owner, get_building_cost, get_required_mass,
};

#[derive(Deserialize)]
pub struct BuildingRequest {
    pub star_x: i32,
    pub star_y: i32,
    pub planet_index: i16,
    pub slot_index: i16,
    pub kind: BuildingKind,
    pub level: i32,
}

pub async fn build_building(
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
    Json(req): Json<BuildingRequest>,
) -> Result<Json<Building>, (StatusCode, String)> {
    let owner_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid user ID in token".to_string(),
        )
    })?;

    // 1. Check for player presence in the system
    let is_present = check_presence(&state.pool, owner_id, req.star_x, req.star_y)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !is_present {
        return Err((
            StatusCode::FORBIDDEN,
            "Player has no presence in this star system".to_string(),
        ));
    }

    let existing_building = sqlx::query_scalar!(
        r#"
    SELECT EXISTS(
        SELECT 1 FROM buildings
        WHERE star_x = $1
        AND star_y = $2
        AND planet_index = $3
        AND slot_index = $4
    )"#,
        req.star_x,
        req.star_y,
        req.planet_index,
        req.slot_index
    )
    .fetch_one(&state.pool)
    .await
    .unwrap_or(None);

    if existing_building.is_some() {
        return Err((
            StatusCode::CONFLICT,
            "There is already a buildling at this location".to_string(),
        ));
    }

    // 2. Get the heaviest ship in the system owned by the player
    // Note: We only consider ships that are NOT in transit.
    let max_mass: Option<f64> = sqlx::query_scalar!(
        r#"
        SELECT MAX((stats->'size_kt')::text::double precision)
        FROM ships 
        WHERE owner_id = $1 AND star_x = $2 AND star_y = $3 AND in_transit = FALSE
        "#,
        owner_id,
        req.star_x,
        req.star_y
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let required_mass = get_required_mass(req.level);

    match max_mass {
        Some(mass) if mass >= required_mass => {
            // Requirements met
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "Required ship mass {}kt not met. Heaviest ship: {}kt",
                    required_mass,
                    max_mass.unwrap_or(0.0)
                ),
            ));
        }
    }

    // 3. Check and deduct costs
    let sales_depot_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM buildings WHERE owner_id = $1 AND kind = 'SalesDepot'",
        owner_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .unwrap_or(0);

    let cost = get_building_cost(req.kind.clone(), req.level, sales_depot_count);

    let user_credits: i64 = sqlx::query_scalar("SELECT credits FROM users WHERE id = $1")
        .bind(owner_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if user_credits < cost {
        return Err((
            StatusCode::PAYMENT_REQUIRED,
            format!("Insufficient credits. Need {}, have {}", cost, user_credits),
        ));
    }

    // 4. Insert or update building within a transaction
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Deduct credits
    sqlx::query("UPDATE users SET credits = credits - $1 WHERE id = $2")
        .bind(cost)
        .bind(owner_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let health = if building_has_health(&req.kind) {
        100 * req.level
    } else {
        100 // Or some default
    };

    let effective_owner = if building_has_owner(&req.kind) {
        Some(owner_id)
    } else {
        None
    };

    let building = sqlx::query_as::<_, Building>(
        r#"
        INSERT INTO buildings (star_x, star_y, planet_index, slot_index, kind, level, owner_id, health)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (star_x, star_y, planet_index, slot_index) 
        DO UPDATE SET 
            kind = EXCLUDED.kind,
            level = EXCLUDED.level,
            owner_id = EXCLUDED.owner_id,
            health = EXCLUDED.health
        RETURNING *
        "#,
    )
    .bind(req.star_x)
    .bind(req.star_y)
    .bind(req.planet_index)
    .bind(req.slot_index)
    .bind(req.kind)
    .bind(req.level)
    .bind(effective_owner)
    .bind(health)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(building))
}
