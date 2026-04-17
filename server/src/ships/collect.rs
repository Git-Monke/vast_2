use std::collections::HashMap;

use axum::{Extension, Json, extract::State, http::StatusCode};
use sqlx::types::Json as SqlxJson;
use uuid::Uuid;

use crate::auth::Claims;
use crate::presence::check_enemy_garrison;
use crate::stock::logic::settle_star_system_stock;
use crate::types::{AppState, Ship};
use universe::material_stock::{
    get_amount, merge_add_kt, merge_into_cargo, normalize_material_vec, total_kt,
};
use universe::resources::baseline_credits_per_kt;
use universe::{Material, MaterialKind};

#[derive(serde::Deserialize)]
pub struct CollectRequest {
    /// Optional map of material kinds to amounts (None amount = take as much as possible)
    pub materials: Option<HashMap<String, Option<f64>>>,
}

#[derive(serde::Serialize)]
pub struct CollectResponse {
    pub collected: Vec<Material>,
    pub remaining_capacity_kt: f64,
}

/// Parse a string like "Iron" or "Helium" into MaterialKind
fn parse_material_kind(s: &str) -> Option<MaterialKind> {
    match s {
        "Iron" => Some(MaterialKind::Iron),
        "Helium" => Some(MaterialKind::Helium),
        _ => None,
    }
}

type HandlerResult<T> = Result<T, (StatusCode, String)>;

pub async fn collect_from_stock(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    axum::extract::Path(ship_id): axum::extract::Path<i64>,
    Json(req): Json<CollectRequest>,
) -> HandlerResult<Json<CollectResponse>> {
    let owner_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    // Fetch the ship
    let ship = sqlx::query_as::<sqlx::Postgres, Ship>(
        "SELECT * FROM ships WHERE id = $1 AND owner_id = $2",
    )
    .bind(ship_id)
    .bind(owner_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Ship not found".to_string()))?;

    // Cannot collect while warping
    if ship.is_warping() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cannot collect cargo while warping".to_string(),
        ));
    }

    let star_x = ship.star_x;
    let star_y = ship.star_y;

    // Check for enemy garrison presence
    if let Some(_) = check_enemy_garrison(&state.pool, owner_id, star_x, star_y)
        .await
        .map_err(|e: crate::error::AppError| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        return Err((
            StatusCode::FORBIDDEN,
            "Cannot collect from a system with enemy garrison presence".to_string(),
        ));
    }

    // Settle the system stock to get accurate amounts
    settle_star_system_stock(&state.pool, star_x, star_y)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Fetch system stock using the same pattern as stock/logic.rs
    let stock_row = sqlx::query!(
        r#"
        SELECT settled
        FROM star_system_stock
        WHERE star_x = $1 AND star_y = $2
        "#,
        star_x,
        star_y
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut system_stock: Vec<Material> = match stock_row {
        Some(ref r) => {
            let json: SqlxJson<Vec<Material>> =
                serde_json::from_value(r.settled.clone()).unwrap_or(SqlxJson(vec![]));
            json.0
        }
        None => vec![],
    };
    normalize_material_vec(&mut system_stock);

    // Get ship cargo capacity
    let max_capacity_kt = ship.stats.size_kt as f64;
    let mut current_cargo: Vec<Material> = ship.cargo.0;
    let current_used_kt = total_kt(&current_cargo);
    let remaining_capacity_kt = (max_capacity_kt - current_used_kt).max(0.0);

    // Nothing to collect if no capacity
    if remaining_capacity_kt < 1e-9 {
        return Err((StatusCode::BAD_REQUEST, "Ship cargo is full".to_string()));
    }

    let mut collected: Vec<Material> = Vec::new();

    if let Some(requested) = req.materials {
        // Specific materials requested
        let mut remaining_cap = remaining_capacity_kt;

        for (kind_str, amount_opt) in requested {
            let kind = parse_material_kind(&kind_str).ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Unknown material: {}", kind_str),
                )
            })?;

            if remaining_cap < 1e-9 {
                break;
            }

            let available = get_amount(&system_stock, kind);
            let requested_amount = amount_opt.unwrap_or(available);

            // Take as much as we can: min(requested, available, remaining_cap)
            let to_take = requested_amount.min(available).min(remaining_cap);

            if to_take > 1e-9 {
                // Take from system stock
                merge_add_kt(&mut system_stock, kind, -to_take);

                // Add to collected
                merge_add_kt(&mut collected, kind, to_take);

                remaining_cap -= to_take;
            }
        }
    } else {
        // No body: take as much as possible prioritizing valuable items
        let mut remaining_cap = remaining_capacity_kt;

        // Sort materials by value descending (Helium first, then Iron)
        let mut kinds: Vec<MaterialKind> = Vec::from(MaterialKind::ALL);
        kinds.sort_by(|a, b| {
            let val_a = baseline_credits_per_kt(*a);
            let val_b = baseline_credits_per_kt(*b);
            val_b.cmp(&val_a) // descending by value
        });

        for kind in kinds {
            if remaining_cap < 1e-9 {
                break;
            }

            let available = get_amount(&system_stock, kind);
            let to_take = available.min(remaining_cap);

            if to_take > 1e-9 {
                // Take from system stock
                merge_add_kt(&mut system_stock, kind, -to_take);

                // Add to collected
                merge_add_kt(&mut collected, kind, to_take);

                remaining_cap -= to_take;
            }
        }
    }

    // If nothing collected, return early
    if collected.is_empty() || total_kt(&collected) < 1e-9 {
        return Ok(Json(CollectResponse {
            collected: vec![],
            remaining_capacity_kt,
        }));
    }

    // Persist the changes

    // Update system stock in DB
    let stock_json = serde_json::to_value(&system_stock)
        .map_err(|e: serde_json::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    sqlx::query("UPDATE star_system_stock SET settled = $1 WHERE star_x = $2 AND star_y = $3")
        .bind(stock_json)
        .bind(star_x)
        .bind(star_y)
        .execute(&state.pool)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Update ship cargo
    merge_into_cargo(&mut current_cargo, &collected);
    let cargo_json = serde_json::to_value(&current_cargo)
        .map_err(|e: serde_json::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    sqlx::query("UPDATE ships SET cargo = $1 WHERE id = $2")
        .bind(cargo_json)
        .bind(ship_id)
        .execute(&state.pool)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let final_used_kt = total_kt(&current_cargo);
    let final_remaining = (max_capacity_kt - final_used_kt).max(0.0);

    Ok(Json(CollectResponse {
        collected,
        remaining_capacity_kt: final_remaining,
    }))
}
