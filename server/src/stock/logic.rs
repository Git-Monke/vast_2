use crate::buildings::prices::{get_mining_rate_kt_s, get_warehouse_capacity_kt};
use crate::types::{Building, BuildingKind};
use sqlx::PgPool;
use sqlx::types::Json;
use universe::Material;
use universe::MaterialKind;
use universe::material_stock::{accrue_settled, mining_rates_hash_from_pairs};

fn parse_material_kind(s: &str) -> Option<MaterialKind> {
    match s {
        "Iron" => Some(MaterialKind::Iron),
        "Helium" => Some(MaterialKind::Helium),
        _ => None,
    }
}

pub async fn settle_star_system_stock(
    tx: &PgPool,
    star_x: i32,
    star_y: i32,
) -> Result<(), sqlx::Error> {
    // Fetch current stock, or default if missing
    let row = sqlx::query!(
        r#"
        SELECT last_settled_at, settled
        FROM star_system_stock
        WHERE star_x = $1 AND star_y = $2
        "#,
        star_x,
        star_y
    )
    .fetch_optional(tx)
    .await?;

    let now = time::OffsetDateTime::now_utc();
    let last_settled_at = row.as_ref().map(|r| r.last_settled_at).unwrap_or(now);

    let mut settled: Vec<Material> = match row {
        Some(ref r) => {
            let json: Json<Vec<Material>> =
                serde_json::from_value(r.settled.clone()).unwrap_or(Json(vec![]));
            json.0
        }
        None => vec![],
    };

    // Fetch buildings to calculate capacity and rates
    let buildings = sqlx::query_as!(
        Building,
        r#"
        SELECT id, star_x, star_y, planet_index, slot_index, kind as "kind: _", level, degradation_percent, mining_material, owner_id, attack_mode as "attack_mode: _", health
        FROM buildings
        WHERE star_x = $1 AND star_y = $2
        "#,
        star_x,
        star_y
    )
    .fetch_all(tx)
    .await?;

    let mut total_capacity_kt = 0.0;
    let mut mining_pairs = Vec::new();

    for b in buildings {
        match b.kind {
            BuildingKind::Warehouse => {
                total_capacity_kt += get_warehouse_capacity_kt(b.level);
            }
            BuildingKind::MiningDepot => {
                if let Some(mat_str) = b.mining_material {
                    if let Some(kind) = parse_material_kind(&mat_str) {
                        mining_pairs.push((kind, get_mining_rate_kt_s(b.level)));
                    }
                }
            }
            _ => {}
        }
    }

    let rates = mining_rates_hash_from_pairs(mining_pairs.into_iter());

    // t_eff in seconds
    let duration = now - last_settled_at;
    let t_eff = duration.as_seconds_f64().max(0.0);

    accrue_settled(&mut settled, &rates, t_eff, total_capacity_kt);

    let settled_json = serde_json::to_value(settled).unwrap_or(serde_json::Value::Array(vec![]));

    // Upsert the record
    let loc_id = universe::star_id::star_location_id(star_x, star_y);
    let loc_id_str = loc_id.to_string();

    // Use sqlx::query instead of query! to avoid compile time errors about the NUMERIC conversion missing bigdecimal feature
    sqlx::query(
        r#"
        INSERT INTO star_system_stock (star_location_id, star_x, star_y, last_settled_at, settled)
        VALUES ($1::numeric, $2, $3, $4, $5)
        ON CONFLICT (star_location_id) DO UPDATE
        SET last_settled_at = EXCLUDED.last_settled_at,
            settled = EXCLUDED.settled
        "#,
    )
    .bind(loc_id_str)
    .bind(star_x)
    .bind(star_y)
    .bind(now)
    .bind(settled_json)
    .execute(tx)
    .await?;

    Ok(())
}
