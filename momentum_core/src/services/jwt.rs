//! JWT Token Service - Pure Rust, no HTTP dependencies
//!
//! This module provides JWT token generation and verification without
//! any HTTP/web framework dependencies.

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::AuthConfig;
use crate::db::models::auth::AuthUser;

/// JWT Claims for access tokens
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: uuid::Uuid, // user_id
    pub email: String,
    pub username: String,
    pub exp: u64,    // expiration time
    pub iat: u64,    // issued at
    pub jti: String, // JWT ID
}

/// JWT Claims for refresh tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: uuid::Uuid, // user_id
    pub exp: u64,        // expiration time
    pub iat: u64,       // issued at
    pub jti: String,    // JWT ID
}

/// JWT Token Service for generating and verifying tokens
#[derive(Clone)]
pub struct JwtService {
    config: AuthConfig,
}

impl JwtService {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    pub fn from_config(config: &crate::config::Config) -> Self {
        Self {
            config: config.auth(),
        }
    }

    /// Generate an access token for a user
    pub fn generate_access_token(
        &self,
        user: &AuthUser,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            sub: user.id,
            email: user.email.clone(),
            username: user.username.clone(),
            exp: now + self.config.access_token_expires_in,
            iat: now,
            jti: uuid::Uuid::new_v4().to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_ref()),
        )
    }

    /// Generate a refresh token for a user
    pub fn generate_refresh_token(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let refresh_expires_in = self.config.refresh_token_expires_in;

        let claims = RefreshClaims {
            sub: user_id,
            exp: now + refresh_expires_in,
            iat: now,
            jti: uuid::Uuid::new_v4().to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_ref()),
        )
    }

    /// Verify and decode an access token
    pub fn verify_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_ref()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }

    /// Verify and decode a refresh token
    pub fn verify_refresh_token(
        &self,
        token: &str,
    ) -> Result<RefreshClaims, jsonwebtoken::errors::Error> {
        let token_data = decode::<RefreshClaims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_ref()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }

    /// Check if a token needs refreshing (expiring within 15 minutes)
    pub fn should_refresh_token(&self, claims: &Claims) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let time_until_expiry = claims.exp.saturating_sub(now);
        time_until_expiry <= 15 * 60
    }

    /// Check if a token is expiring soon (within 5 minutes)
    pub fn is_token_expiring_soon(&self, claims: &Claims) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let time_until_expiry = claims.exp.saturating_sub(now);
        time_until_expiry <= 5 * 60
    }
}

impl Default for JwtService {
    fn default() -> Self {
        Self::new(AuthConfig {
            jwt_secret: "default-secret".to_string(),
            access_token_expires_in: 3600,
            refresh_token_expires_in: 604800,
        })
    }
}
