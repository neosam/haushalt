use actix_web::HttpRequest;
use uuid::Uuid;

use crate::services::auth as auth_service;

/// JWT Claims structure
#[derive(Debug)]
pub struct TokenClaims {
    pub sub: Uuid,
}

/// Validate a JWT token and return claims
pub fn validate_token(token: &str, jwt_secret: &str) -> Result<TokenClaims, AuthMiddlewareError> {
    auth_service::verify_jwt(token, jwt_secret)
        .map(|sub| TokenClaims { sub })
        .map_err(|_| AuthMiddlewareError::InvalidToken)
}

/// Extract user ID from the Authorization header
pub fn extract_user_id(req: &HttpRequest, jwt_secret: &str) -> Result<Uuid, AuthMiddlewareError> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .ok_or(AuthMiddlewareError::MissingToken)?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| AuthMiddlewareError::InvalidToken)?;

    if !auth_str.starts_with("Bearer ") {
        return Err(AuthMiddlewareError::InvalidToken);
    }

    let token = &auth_str[7..];

    auth_service::verify_jwt(token, jwt_secret)
        .map_err(|_| AuthMiddlewareError::InvalidToken)
}

#[derive(Debug)]
pub enum AuthMiddlewareError {
    MissingToken,
    InvalidToken,
}

impl std::fmt::Display for AuthMiddlewareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthMiddlewareError::MissingToken => write!(f, "Missing authorization token"),
            AuthMiddlewareError::InvalidToken => write!(f, "Invalid authorization token"),
        }
    }
}

impl std::error::Error for AuthMiddlewareError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_display() {
        assert_eq!(
            AuthMiddlewareError::MissingToken.to_string(),
            "Missing authorization token"
        );
        assert_eq!(
            AuthMiddlewareError::InvalidToken.to_string(),
            "Invalid authorization token"
        );
    }
}
