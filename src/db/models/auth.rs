use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// User models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone, Debug)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub name: String,
    pub email: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub id: Uuid,
    pub current_workspace_id: Option<Uuid>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser {
    pub email: String,
    pub username: String,
    pub name: String,
    pub avatar_url: Option<String>,
}

// User Credential models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::user_credentials)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserCredential {
    pub id: i32,
    pub user_id: Uuid,
    pub credential_type: String,
    pub credential_hash: Option<String>,
    pub oauth_provider_id: Option<String>,
    pub oauth_user_id: Option<String>,
    pub is_primary: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::user_credentials)]
pub struct NewUserCredential {
    pub user_id: Uuid,
    pub credential_type: String,
    pub credential_hash: Option<String>,
    pub oauth_provider_id: Option<String>,
    pub oauth_user_id: Option<String>,
    pub is_primary: bool,
}

// Authentication DTOs
#[derive(Serialize)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub name: String,
    pub avatar_url: Option<String>,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub username: String,
    pub name: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: AuthUser,
}

#[derive(Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct UserBasicInfo {
    pub id: Uuid,
    pub name: String,
    pub username: String,
    pub email: String,
    pub avatar_url: Option<String>,
}

#[derive(Serialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub current_workspace_id: Option<Uuid>,
    pub workspaces: Vec<super::workspace::WorkspaceInfo>,
    pub teams: Vec<super::team::TeamInfo>,
}
