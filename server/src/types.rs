use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use universe::{Material, ShipAttackMode, ShipStats};
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
