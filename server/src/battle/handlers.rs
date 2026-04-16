use axum::{Extension, Json, extract::Path, http::StatusCode, extract::State};
use serde::Deserialize;
use uuid::Uuid;

use crate::{auth, battle::logic::execute_battle, types::AppState, error::AppError};

#[derive(Deserialize)]
pub struct BattleRequest {
    /// The empire that initiates the battle (will be taken from JWT claims).
    #[serde(skip)]
    pub initiator: Uuid,
}

#[derive(serde::Serialize)]
pub struct BattleResponse {
    pub result: crate::battle::logic::BattleExecutionResult,
}

pub async fn battle_handler(
    Extension(claims): Extension<auth::Claims>,
    State(state): State<AppState>,
    Path((x, y)): Path<(i32, i32)>,
) -> Result<Json<BattleResponse>, (StatusCode, String)> {
    let initiator = Uuid::parse_str(&claims.sub).map_err(|_| {
        (StatusCode::BAD_REQUEST, "Invalid user ID in token".to_string())
    })?;

    let result = execute_battle(&state.pool, x, y, initiator)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(BattleResponse { result }))
}
