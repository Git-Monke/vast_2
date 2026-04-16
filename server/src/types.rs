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

#[derive(Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct StarSystemStock {
    pub star_x: i32,
    pub star_y: i32,
    pub last_settled_at: time::OffsetDateTime,
    pub capacity_kt: f64,
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
    pub ships: Vec<Ship>,
}
