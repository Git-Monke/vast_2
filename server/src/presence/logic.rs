use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

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
            WHERE owner_id = $1 AND star_x = $2 AND star_y = $3 AND (warp_completed_at IS NULL OR warp_completed_at <= NOW())
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

/// Returns true if any *enemy* ship or building in the given system has
/// `attack_mode = 'StrikeFirst'`. An enemy is defined as any entity whose
/// `owner_id` differs from the supplied `owner_id`.
pub async fn check_enemy_strike_first(
    pool: &PgPool,
    star_x: i32,
    star_y: i32,
    owner_id: Uuid,
) -> Result<bool, AppError> {
    // Look for a ship with StrikeFirst owned by a different empire.
    // Also look for a building with StrikeFirst owned by a different empire.
    let row = sqlx::query!(
        r#"
        SELECT 1 as exists FROM ships
        WHERE star_x = $1 AND star_y = $2 AND owner_id <> $3 AND attack_mode = 'StrikeFirst'
        UNION ALL
        SELECT 1 FROM buildings
        WHERE star_x = $1 AND star_y = $2 AND owner_id IS NOT NULL AND owner_id <> $3 AND attack_mode = 'StrikeFirst'
        LIMIT 1
        "#,
        star_x,
        star_y,
        owner_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

/// Returns the owner_id of an enemy garrison (MilitaryGarrison) if one exists in the given system.
/// If no enemy garrison is present, returns `Ok(None)`.
/// Errors are wrapped in `AppError`.
pub async fn check_enemy_garrison(
    pool: &PgPool,
    empire_id: Uuid,
    star_x: i32,
    star_y: i32,
) -> Result<Option<Uuid>, AppError> {
    // Look for any MilitaryGarrison building owned by a different empire.
    let row = sqlx::query!(
        r#"
        SELECT owner_id FROM buildings
        WHERE kind = 'MilitaryGarrison' AND star_x = $1 AND star_y = $2 AND owner_id <> $3
        LIMIT 1
        "#,
        star_x,
        star_y,
        empire_id
    )
    .fetch_optional(pool)
    .await?;

    // `owner_id` is nullable, so we need to flatten the Option<Option<Uuid>>.
    Ok(row.and_then(|r| r.owner_id))
}
