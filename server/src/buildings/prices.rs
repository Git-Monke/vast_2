use crate::types::{BuildingKind, Ship};

/// Returns the resource cost for a building of a certain type and level.
pub fn get_building_cost(kind: BuildingKind, level: i32, sales_depot_count: i64) -> i64 {
    if kind == BuildingKind::SalesDepot {
        return 1000 * 2i64.pow(sales_depot_count as u32);
    }

    // Exponential-ish scaling for level 1-10
    // level 1: 100
    // level 2: 200
    // level 4: 1000 (roughly)
    // A power of 2.15 handles this reasonably: 100 * 2.15^(level-1)
    let base = 100.0;
    let growth = 2.15f64;
    (base * growth.powi(level - 1)) as i64
}

/// Returns the required ship mass in kilotons to build/upgrade to this level.
/// level 1: any ship (0kt requirement)
/// level 5: 50kt
/// level 10: 1000kt
pub fn get_required_mass(level: i32) -> f64 {
    if level <= 1 {
        0.0
    } else if level >= 10 {
        1000.0
    } else {
        match level {
            2 => 5.0,
            3 => 15.0,
            4 => 30.0,
            5 => 50.0,
            6 => 100.0,
            7 => 200.0,
            8 => 400.0,
            9 => 700.0,
            _ => 1000.0,
        }
    }
}

pub fn building_has_owner(kind: &BuildingKind) -> bool {
    matches!(
        kind,
        BuildingKind::MilitaryGarrison | BuildingKind::Radar | BuildingKind::SalesDepot
    )
}

pub fn building_has_health(kind: &BuildingKind) -> bool {
    matches!(kind, BuildingKind::MilitaryGarrison)
}

/// Returns warehouse capacity in kilotons for a given level.
pub fn get_warehouse_capacity_kt(level: i32) -> f64 {
    10.0 * (level as f64).powf(2.0)
}

/// Returns mining rate in kilotons per second for a given level.
pub fn get_mining_rate_kt_s(level: i32) -> f64 {
    (1.0 * (level as f64).powf(1.5)) / 100.0
}

pub fn get_ship_depot_capacity_kt(level: i32) -> u32 {
    (10.0 * f32::powf(10.0, (level as f32 - 1.0) / 4.5)) as u32
}

pub async fn get_depot_used_capacity_kt(
    pool: &sqlx::PgPool,
    building_id: i64,
) -> Result<u32, sqlx::Error> {
    // Note: Later on, this must also sum the mass of ships currently under construction at this depot.
    let ships = sqlx::query_as::<sqlx::Postgres, Ship>("SELECT * FROM ships WHERE docked_at = $1")
        .bind(building_id)
        .fetch_all(pool)
        .await?;

    let used_kt: u32 = ships.iter().map(|s| s.stats.size_kt).sum();
    Ok(used_kt)
}
