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
