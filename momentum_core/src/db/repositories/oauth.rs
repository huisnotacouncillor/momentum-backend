use diesel::prelude::*;
use crate::error::AppError;
use crate::schema::oauth_providers;

pub struct OAuthRepo;

impl OAuthRepo {
    pub fn find_by_provider_name(
        conn: &mut PgConnection,
        provider_name: &str,
    ) -> Result<Option<OAuthProvider>, AppError> {
        oauth_providers::table
            .filter(oauth_providers::provider_name.eq(provider_name))
            .filter(oauth_providers::is_active.eq(true))
            .first(conn)
            .optional()
            .map_err(|e| AppError::Internal(format!("Failed to find OAuth provider: {}", e)))
    }
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = oauth_providers)]
pub struct OAuthProvider {
    pub id: i32,
    pub provider_name: String,
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub user_info_url: String,
    pub scope: Option<String>,
    pub is_active: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub redirect_uri: Option<String>,
}