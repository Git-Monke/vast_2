use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth,
    presence::check_presence,
    types::{AppState, Building, BuildingKind},
};

use super::prices::{get_building_cost, get_required_mass};

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

    // 2. Get the heaviest ship in the system owned by the player
    // Note: We only consider ships that are NOT in transit.
    let max_mass: Option<f64> = sqlx::query_scalar!(
        r#"
        SELECT MAX((stats->'mass_kt')::text::double precision)
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

    // 3. (Scaffold) Check and deduct costs
    let _cost = get_building_cost(req.kind.clone(), req.level);
    // TODO: Actually check credits/materials when that system is ready

    // 4. Insert or update building
    // Initial health could be based on level or building kind, setting 100 for now.
    let health = 100 * req.level;

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
    .bind(owner_id)
    .bind(health)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(building))
}
