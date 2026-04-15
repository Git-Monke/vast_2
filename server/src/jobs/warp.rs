use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WarpJob {
    pub scheduled_id: i64,
    pub scheduled_at: chrono::DateTime<chrono::Utc>,
    pub ship_id: i64,
    pub to_star_x: i32,
    pub to_star_y: i32,
}
