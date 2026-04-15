use crate::auth::Claims;
use crate::types::AppState;
use axum::{Extension, extract::Path, extract::State};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WarpJob {
    pub scheduled_id: i64,
    pub scheduled_at: OffsetDateTime,
    pub ship_id: i64,
    pub to_star_x: i32,
    pub to_star_y: i32,
}

pub async fn warp_ship_handler(
    Path(id): Path<u64>,
    Extension(_claims): Extension<Claims>,
    _state: State<AppState>,
) -> String {
    format!("hello {}", id)
}
