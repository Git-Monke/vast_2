use jsonwebtoken::{
    DecodingKey, EncodingKey, Header, Validation, decode, encode, errors::Error as JwtError,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}

/// Returns the JWT secret as a byte slice. Panics if `JWT_SECRET` is not set.
static JWT_SECRET: Lazy<Vec<u8>> = Lazy::new(|| {
    std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set in the environment")
        .into_bytes()
});

fn secret_key() -> &'static [u8] {
    &JWT_SECRET
}

/// Create a JWT for the given claims using the configured secret.
/// Returns the token string or a jsonwebtoken::Error.
pub fn create_token(claims: &Claims) -> Result<String, JwtError> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret_key()),
    )
}

/// Decode and validate a JWT using the configured secret.
/// Returns the decoded claims or a jsonwebtoken::Error.
pub fn decode_token(token: &str) -> Result<Claims, JwtError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret_key()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

/// Helper to generate default claims for a user.
#[allow(dead_code)]
pub fn default_claims(user_id: &Uuid, username: String, ttl_secs: usize) -> Claims {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as usize;
    Claims {
        sub: user_id.to_string(),
        username,
        iat: now,
        exp: now + ttl_secs,
    }
}
