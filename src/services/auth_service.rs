use diesel::prelude::*;
use bcrypt::{hash, verify};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    db::models::auth::{User, NewUser, NewUserCredential, RegisterRequest, LoginRequest, LoginResponse, UserProfile, AuthUser},
    db::repositories::auth::AuthRepo,
    error::AppError,
    services::context::RequestContext,
    validation::auth::{validate_register_request, validate_login_request, validate_update_profile, UpdateProfileChanges},
};

pub struct AuthService;

impl AuthService {
    pub fn register(
        conn: &mut PgConnection,
        req: &RegisterRequest,
    ) -> Result<User, AppError> {
        validate_register_request(&req.name, &req.username, &req.email, &req.password)?;

        // Check if email already exists
        if AuthRepo::exists_by_email(conn, &req.email)? {
            return Err(AppError::conflict_with_code(
                "Email already exists",
                Some("email".to_string()),
                "USER_EMAIL_EXISTS",
            ));
        }

        // Check if username already exists
        if AuthRepo::exists_by_username(conn, &req.username)? {
            return Err(AppError::conflict_with_code(
                "Username already exists",
                Some("username".to_string()),
                "USER_USERNAME_EXISTS",
            ));
        }

        let _now = Utc::now().naive_utc();
        let user_id = Uuid::new_v4();

        // Hash password
        let hashed_password = hash(&req.password, bcrypt::DEFAULT_COST)
            .map_err(|_| AppError::internal("Failed to hash password"))?;

        let new_user = NewUser {
            email: req.email.clone(),
            username: req.username.clone(),
            name: req.name.clone(),
            avatar_url: None,
        };

        let user = AuthRepo::insert_user(conn, &new_user)?;

        // Create credential
        let new_credential = NewUserCredential {
            user_id,
            credential_type: "password".to_string(),
            credential_hash: Some(hashed_password),
            oauth_provider_id: None,
            oauth_user_id: None,
            is_primary: true,
        };

        AuthRepo::insert_credential(conn, &new_credential)?;

        Ok(user)
    }

    pub fn login(
        conn: &mut PgConnection,
        req: &LoginRequest,
    ) -> Result<LoginResponse, AppError> {
        validate_login_request(&req.email, &req.password)?;

        let user = AuthRepo::find_by_email(conn, &req.email)?
            .ok_or_else(|| AppError::auth("Invalid email or password"))?;

        let credential = AuthRepo::find_credential_by_user_id(conn, user.id)?
            .ok_or_else(|| AppError::auth("Invalid email or password"))?;

        let is_valid = verify(&req.password, &credential.credential_hash.as_ref().unwrap())
            .map_err(|_| AppError::internal("Failed to verify password"))?;

        if !is_valid {
            return Err(AppError::auth("Invalid email or password"));
        }

        // Generate JWT token (simplified - in real app you'd use proper JWT library)
        let token = format!("token_{}", user.id);

        Ok(LoginResponse {
            access_token: token,
            refresh_token: "refresh_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            user: AuthUser {
                id: user.id,
                email: user.email.clone(),
                username: user.username.clone(),
                name: user.name.clone(),
                avatar_url: user.avatar_url.clone(),
            },
        })
    }

    pub fn get_profile(
        conn: &mut PgConnection,
        ctx: &RequestContext,
    ) -> Result<UserProfile, AppError> {
        let user = AuthRepo::find_by_id(conn, ctx.user_id)?
            .ok_or_else(|| AppError::not_found("user"))?;

        Ok(UserProfile {
            id: user.id,
            name: user.name,
            username: user.username,
            email: user.email,
            avatar_url: user.avatar_url,
            current_workspace_id: user.current_workspace_id,
            workspaces: vec![],
            teams: vec![],
        })
    }

    pub fn update_profile(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        changes: &crate::routes::auth::UpdateProfileRequest,
    ) -> Result<UserProfile, AppError> {
        // Validate changes
        let update_changes = UpdateProfileChanges {
            name: changes.name.as_deref(),
            username: changes.username.as_deref(),
            email: changes.email.as_deref(),
            avatar_url: changes.avatar_url.as_deref(),
        };
        validate_update_profile(&update_changes)?;

        // Check username uniqueness if username changes
        if let Some(ref new_username) = changes.username {
            if let Some(existing_user) = AuthRepo::find_by_username(conn, new_username)? {
                if existing_user.id != ctx.user_id {
                    return Err(AppError::conflict_with_code(
                        "Username already exists",
                        Some("username".to_string()),
                        "USER_USERNAME_EXISTS",
                    ));
                }
            }
        }

        // Check email uniqueness if email changes
        if let Some(ref new_email) = changes.email {
            if let Some(existing_user) = AuthRepo::find_by_email(conn, new_email)? {
                if existing_user.id != ctx.user_id {
                    return Err(AppError::conflict_with_code(
                        "Email already exists",
                        Some("email".to_string()),
                        "USER_EMAIL_EXISTS",
                    ));
                }
            }
        }

        let updated_user = AuthRepo::update_user_fields(
            conn,
            ctx.user_id,
            (
                changes.name.clone(),
                changes.username.clone(),
                changes.email.clone(),
                changes.avatar_url.clone(),
            ),
        )?;

        Ok(UserProfile {
            id: updated_user.id,
            name: updated_user.name,
            username: updated_user.username,
            email: updated_user.email,
            avatar_url: updated_user.avatar_url,
            current_workspace_id: updated_user.current_workspace_id,
            workspaces: vec![],
            teams: vec![],
        })
    }

    pub fn switch_workspace(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        workspace_id: Uuid,
    ) -> Result<User, AppError> {
        // Verify user has access to the workspace (this would need workspace member check)
        // For now, just update the current workspace
        let updated_user = AuthRepo::update_current_workspace(conn, ctx.user_id, workspace_id)?;
        Ok(updated_user)
    }
}