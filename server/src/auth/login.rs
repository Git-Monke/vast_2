use crate::types::AppState;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{Json, extract::State, http::StatusCode};

use crate::auth::jwt::{Claims, create_token, decode_token};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: String,
}



pub async fn authorize(
    State(state): State<AppState>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    // 1. Fetch user from DB
    let (user_id, username, password_hash): (Uuid, String, String) =
        sqlx::query_as("SELECT id, username, password_hash FROM users WHERE username = $1")
            .bind(&payload.username)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error".into()))?
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid credentials".into()))?;

    // 2. Verify password
    let parsed_hash = PasswordHash::new(&password_hash).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Invalid password hash format".into(),
        )
    })?;

    Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid credentials".into()))?;

    // 3. Generate JWT
    let now = time::OffsetDateTime::now_utc().unix_timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        username,
        exp: now + 3600, // 1 hour
        iat: now,
    };

    let token = create_token(&claims)
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create token".into(),
        ))?;

    Ok(Json(AuthResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
    }))
}

pub async fn auth_middleware(
    State(_state): State<AppState>,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, (StatusCode, String)> {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Missing authorization header".into(),
        ))?;

    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid authorization header".into(),
        ));
    }

    let token = &auth_header[7..];

    let token_data = decode_token(token)
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token".into()))?;

    req.extensions_mut().insert(token_data);

    Ok(next.run(req).await)
}
