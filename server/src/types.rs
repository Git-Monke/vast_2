use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use universe::{Material, ShipAttackMode, ShipStats, generator::StarSystem};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
}

#[derive(Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct Ship {
    pub id: i64,
    pub owner_id: Uuid,
    pub stats: Json<ShipStats>,
    pub cargo: Json<Vec<Material>>,
    pub attack_mode: ShipAttackMode,
    pub in_transit: bool,
    pub star_x: i32,
    pub star_y: i32,
    pub jump_ready_at: time::OffsetDateTime,
    pub health: i32,
    pub docked_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "building_kind")]
pub enum BuildingKind {
    MiningDepot,
    Warehouse,
    MilitaryGarrison,
    SalesDepot,
    ShipDepot,
    Radar,
}

#[derive(Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct Building {
    pub id: i64,
    pub star_x: i32,
    pub star_y: i32,
    pub planet_index: i16,
    pub slot_index: i16,
    pub kind: BuildingKind,
    pub level: i32,
    pub degradation_percent: f32,
    pub mining_material: Option<String>,
    pub owner_id: Option<Uuid>,
    pub attack_mode: Option<ShipAttackMode>,
    pub health: i32,
}

#[derive(Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct StarSystemStock {
    pub star_x: i32,
    pub star_y: i32,
    pub last_settled_at: time::OffsetDateTime,
    pub settled: Json<Vec<Material>>,
}

#[derive(Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct PlayerPresence {
    pub id: i64,
    pub star_x: i32,
    pub star_y: i32,
    pub empire_id: Uuid,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StarSystemDetails {
    pub system: StarSystem,
    pub stock: Option<StarSystemStock>,
    pub buildings: Vec<Building>,
    pub ships: Vec<Ship>,
}
