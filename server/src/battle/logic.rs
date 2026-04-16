use crate::error::AppError;
use crate::types::{Building, Ship};
use sqlx::PgPool;
use universe::battle::{self, CombatantData, CombatantId};
use uuid::Uuid;

/// Result of a battle execution
#[derive(Debug, serde::Serialize)]
pub struct BattleExecutionResult {
    pub initiator_damage: u32,
    pub defender_damage: u32,
    pub initiator_dead: bool,
    pub defender_dead: bool,
    pub winner: String,
}

/// Core function that runs a battle in a star system.
///
/// * `pool` – database connection pool.
/// * `star_x`, `star_y` – location of the system.
/// * `initiator` – the empire that initiates the fight.
///
/// Returns a struct with a summary of the outcome.
pub async fn execute_battle(
    tx: &PgPool,
    star_x: i32,
    star_y: i32,
    initiator: Uuid,
) -> Result<BattleExecutionResult, AppError> {
    // 1️⃣ Fetch participants
    let ships = sqlx::query_as::<_, Ship>(
        r#"
        SELECT * FROM ships
        WHERE star_x = $1 AND star_y = $2
          AND (warp_completed_at IS NULL OR warp_completed_at <= NOW())
          AND docked_at IS NULL
        "#,
    )
    .bind(star_x)
    .bind(star_y)
    .fetch_all(tx)
    .await?;

    let garrisons = sqlx::query_as::<_, Building>(
        r#"
        SELECT * FROM buildings
        WHERE star_x = $1 AND star_y = $2 AND kind = 'MilitaryGarrison'
        "#,
    )
    .bind(star_x)
    .bind(star_y)
    .fetch_all(tx)
    .await?;

    // 2️⃣ Partition into teams based on the initiator
    let mut initiator_combatants: Vec<CombatantData> = Vec::new();
    let mut defender_combatants: Vec<CombatantData> = Vec::new();

    for ship in ships {
        let combatant = CombatantData::from_ship(ship.id, ship.stats.0.clone(), ship.health as u32);
        if ship.owner_id == initiator {
            initiator_combatants.push(combatant);
        } else {
            defender_combatants.push(combatant);
        }
    }

    for b in garrisons {
        // Only consider garrisons that have an owner (otherwise they don't belong to a player)
        if let Some(owner) = b.owner_id {
            let combatant = CombatantData::from_garrison(b.id, b.level);
            if owner == initiator {
                initiator_combatants.push(combatant);
            } else {
                defender_combatants.push(combatant);
            }
        }
    }

    // If one side is empty we simply skip battle
    if initiator_combatants.is_empty() || defender_combatants.is_empty() {
        return Ok(BattleExecutionResult {
            initiator_damage: 0,
            defender_damage: 0,
            initiator_dead: false,
            defender_dead: false,
            winner: "stalemate".into(),
        });
    }

    // 3️⃣ Run the battle using the universe logic
    let results = battle::run_battle(&initiator_combatants, &defender_combatants);

    // Helper to apply damage to a ship or building
    async fn apply_damage(
        pool: &PgPool,
        id: i64,
        damage: u32,
        table: &str,
    ) -> Result<bool, AppError> {
        // Fetch current health
        let current: i32 =
            sqlx::query_scalar(&format!("SELECT health FROM {} WHERE id = $1", table))
                .bind(id)
                .fetch_one(pool)
                .await?;
        let new_health = current - damage as i32;
        if new_health > 0 {
            sqlx::query(&format!("UPDATE {} SET health = $1 WHERE id = $2", table))
                .bind(new_health)
                .bind(id)
                .execute(pool)
                .await?;
            Ok(false)
        } else {
            sqlx::query(&format!("DELETE FROM {} WHERE id = $1", table))
                .bind(id)
                .execute(pool)
                .await?;
            Ok(true)
        }
    }

    let mut initiator_dead = false;
    let mut defender_dead = false;
    let mut initiator_damage = 0u32;
    let mut defender_damage = 0u32;

    for res in results {
        match res.id {
            CombatantId::Ship(id) => {
                let dead = apply_damage(tx, id, res.damage_taken, "ships").await?;
                if dead {
                    if initiator_combatants.iter().any(|c| c.id == res.id) {
                        initiator_dead = true;
                    } else {
                        defender_dead = true;
                    }
                }
                if initiator_combatants.iter().any(|c| c.id == res.id) {
                    initiator_damage += res.damage_taken;
                } else {
                    defender_damage += res.damage_taken;
                }
            }
            CombatantId::Garrison(id) => {
                let dead = apply_damage(tx, id, res.damage_taken, "buildings").await?;
                if dead {
                    if initiator_combatants.iter().any(|c| c.id == res.id) {
                        initiator_dead = true;
                    } else {
                        defender_dead = true;
                    }
                }
                if initiator_combatants.iter().any(|c| c.id == res.id) {
                    initiator_damage += res.damage_taken;
                } else {
                    defender_damage += res.damage_taken;
                }
            }
        }
    }

    let winner = if initiator_dead && !defender_dead {
        "defenders"
    } else if defender_dead && !initiator_dead {
        "initiator"
    } else {
        "stalemate"
    };

    Ok(BattleExecutionResult {
        initiator_damage,
        defender_damage,
        initiator_dead,
        defender_dead,
        winner: winner.into(),
    })
}
