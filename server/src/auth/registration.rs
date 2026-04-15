use crate::types::AppState;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use sqlx::types::Json as SQLJson;
use universe::{ShipStats, helpers::find_empty_red_dwarf_starter};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

pub async fn register_user(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // 1. Hash password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to hash password".to_string(),
            )
        })?
        .to_string();

    // 2. Start a transaction to ensure user and ship are created together
    let mut tx = state.pool.begin().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to start transaction: {}", e),
        )
    })?;

    // 3. Save user to DB
    let user_id = Uuid::new_v4();

    sqlx::query("INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(payload.username)
        .bind(password_hash)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if let Some(db_err) = e.as_database_error() {
                if db_err.is_unique_violation() {
                    return (StatusCode::CONFLICT, "Username already exists".to_string());
                }
            }
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?;

    // 4. Find a starting location: a red dwarf with no military garrisons
    let mut star_coords = None;
    for _ in 0..10 {
        if let Some((sx, sy)) = find_empty_red_dwarf_starter() {
            let garrison_exists: bool = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM buildings WHERE star_x = $1 AND star_y = $2 AND kind = 'MilitaryGarrison')"
            )
            .bind(sx)
            .bind(sy)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            if !garrison_exists {
                star_coords = Some((sx, sy));
                break;
            }
        }
    }

    let (star_x, star_y) = star_coords.ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to find a suitable starting star".to_string(),
    ))?;

    // 5. Create a default ship for the user
    let default_stats = ShipStats::default();
    let empty_cargo: Vec<universe::Material> = Vec::new();

    sqlx::query(
        "INSERT INTO ships (owner_id, stats, cargo, attack_mode, star_x, star_y, jump_ready_at, health) \
         VALUES ($1, $2, $3, 'Defend', $4, $5, NOW(), $6)"
    )
    .bind(user_id)
    .bind(SQLJson(default_stats))
    .bind(SQLJson(empty_cargo))
    .bind(star_x)
    .bind(star_y)
    .bind(10) // Default health matching default defense
    .execute(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create starting ship: {}", e)))?;

    tx.commit().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to commit transaction: {}", e),
        )
    })?;

    Ok(StatusCode::CREATED)
}
