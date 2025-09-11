use std::sync::Arc;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use diesel::prelude::*;
use uuid::Uuid;

use crate::{
    config::Config,
    db::{
        models::{
            auth::{AuthUser, LoginResponse, NewUser, NewUserCredential, RegisterRequest, User, UserCredential},
            label::NewLabel,
            team::{NewTeam, NewTeamMember, Team},
            workspace::{NewWorkspace, Workspace},
            workspace_member::{NewWorkspaceMember, WorkspaceMemberRole},
        },
        DbPool,
    },
    db::enums::LabelLevel,
    error::{AppError, AppResult},
    schema,
};

pub struct AuthService {
    pool: Arc<DbPool>,
    config: Arc<Config>,
}

impl AuthService {
    pub fn new(pool: Arc<DbPool>, config: Arc<Config>) -> Self {
        Self { pool, config }
    }

    pub async fn register_user(&self, request: RegisterRequest) -> AppResult<LoginResponse> {
        let mut conn = self.pool.get()?;

        // 检查用户是否存在
        self.check_user_exists(&mut conn, &request).await?;

        // 创建用户（事务处理）
        let user = self.create_user_with_workspace(&mut conn, request).await?;

        // 生成令牌
        self.generate_tokens(&user).await
    }

    pub async fn login_user(&self, email: String, password: String) -> AppResult<LoginResponse> {
        let mut conn = self.pool.get()?;

        // 查找用户
        let user: User = schema::users::table
            .filter(schema::users::email.eq(&email))
            .filter(schema::users::is_active.eq(true))
            .select(User::as_select())
            .first(&mut conn)
            .optional()?
            .ok_or_else(|| AppError::auth("Invalid email or password"))?;

        // 查找用户认证信息
        let credential: UserCredential = schema::user_credentials::table
            .filter(schema::user_credentials::user_id.eq(user.id))
            .filter(schema::user_credentials::credential_type.eq("password"))
            .filter(schema::user_credentials::is_primary.eq(true))
            .select(UserCredential::as_select())
            .first(&mut conn)
            .optional()?
            .ok_or_else(|| AppError::auth("Invalid email or password"))?;

        // 验证密码
        if let Some(hash) = credential.credential_hash {
            let is_valid = verify(password.as_bytes(), &hash)?;
            if !is_valid {
                return Err(AppError::auth("Invalid email or password"));
            }
        } else {
            return Err(AppError::auth("Invalid email or password"));
        }

        // 生成令牌
        self.generate_tokens(&user).await
    }

    pub async fn refresh_token(&self, _refresh_token: String) -> AppResult<LoginResponse> {
        // TODO: 实现 refresh token 验证逻辑
        // 暂时返回错误，后续完善
        Err(AppError::auth("Refresh token not implemented yet"))
    }

    async fn check_user_exists(&self, conn: &mut PgConnection, request: &RegisterRequest) -> AppResult<()> {
        // 检查邮箱是否已存在
        let existing_user = schema::users::table
            .filter(schema::users::email.eq(&request.email))
            .select(User::as_select())
            .first(conn)
            .optional()?;

        if existing_user.is_some() {
            return Err(AppError::conflict_with_code(
                "Email address already exists",
                Some("email".to_string()),
                "USER_EMAIL_EXISTS",
            ));
        }

        // 检查用户名是否已存在
        let existing_username = schema::users::table
            .filter(schema::users::username.eq(&request.username))
            .first::<User>(conn)
            .optional()?;

        if existing_username.is_some() {
            return Err(AppError::conflict_with_code(
                "Username already exists",
                Some("username".to_string()),
                "USER_USERNAME_EXISTS",
            ));
        }

        Ok(())
    }

    async fn create_user_with_workspace(&self, conn: &mut PgConnection, request: RegisterRequest) -> AppResult<User> {
        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            // 创建新用户
            let new_user = NewUser {
                email: request.email.clone(),
                username: request.username.clone(),
                name: request.name.clone(),
                avatar_url: None,
            };

            let user: User = diesel::insert_into(schema::users::table)
                .values(&new_user)
                .get_result(conn)?;

            // 哈希密码
            let password_hash = hash(request.password.as_bytes(), DEFAULT_COST)
                .map_err(|_| diesel::result::Error::RollbackTransaction)?;

            // 创建用户认证记录
            let new_credential = NewUserCredential {
                user_id: user.id,
                credential_type: "password".to_string(),
                credential_hash: Some(password_hash),
                oauth_provider_id: None,
                oauth_user_id: None,
                is_primary: true,
            };

            diesel::insert_into(schema::user_credentials::table)
                .values(&new_credential)
                .execute(conn)?;

            // 创建默认工作空间
            let workspace_name = format!("{}'s Workspace", request.name);
            let workspace_url_key = format!("{}-workspace", request.username.to_lowercase());

            let new_workspace = NewWorkspace {
                name: workspace_name,
                url_key: workspace_url_key,
            };

            let workspace: Workspace = diesel::insert_into(schema::workspaces::table)
                .values(&new_workspace)
                .get_result(conn)?;

            // 创建默认团队
            let team_name = "Default Team".to_string();
            let team_key = "DEF".to_string();

            let new_team = NewTeam {
                workspace_id: workspace.id,
                name: team_name,
                team_key,
                description: None,
                icon_url: None,
                is_private: false,
            };

            let team: Team = diesel::insert_into(schema::teams::table)
                .values(&new_team)
                .get_result(conn)?;

            // 将用户添加为团队成员，角色为 "admin"
            let new_team_member = NewTeamMember {
                user_id: user.id,
                team_id: team.id,
                role: "admin".to_string(),
            };

            diesel::insert_into(schema::team_members::table)
                .values(&new_team_member)
                .execute(conn)?;

            // 将用户添加为工作区成员，角色为 "owner"
            let new_workspace_member = NewWorkspaceMember {
                user_id: user.id,
                workspace_id: workspace.id,
                role: WorkspaceMemberRole::Owner,
            };

            diesel::insert_into(schema::workspace_members::table)
                .values(&new_workspace_member)
                .execute(conn)?;

            // 设置用户的当前workspace为新创建的默认workspace
            diesel::update(schema::users::table)
                .filter(schema::users::id.eq(user.id))
                .set(schema::users::current_workspace_id.eq(Some(workspace.id)))
                .execute(conn)?;

            // 为新创建的工作区添加默认标签
            self.create_default_labels(conn, workspace.id)?;

            Ok(user)
        }).map_err(AppError::Database)
    }

    fn create_default_labels(&self, conn: &mut PgConnection, workspace_id: Uuid) -> Result<(), diesel::result::Error> {
        let now = Utc::now().naive_utc();
        let default_labels = vec![
            NewLabel {
                workspace_id,
                name: "Feature".to_string(),
                color: "#BB8FCE".to_string(),
                level: LabelLevel::Issue,
                created_at: now,
                updated_at: now,
            },
            NewLabel {
                workspace_id,
                name: "Improvement".to_string(),
                color: "#85C1E9".to_string(),
                level: LabelLevel::Issue,
                created_at: now,
                updated_at: now,
            },
            NewLabel {
                workspace_id,
                name: "Bug".to_string(),
                color: "#FF6B6B".to_string(),
                level: LabelLevel::Issue,
                created_at: now,
                updated_at: now,
            },
        ];

        for new_label in default_labels {
            diesel::insert_into(schema::labels::table)
                .values(&new_label)
                .execute(conn)
                .ok(); // 忽略插入失败的情况，保证即使标签创建失败也不会影响注册流程
        }

        Ok(())
    }

    async fn generate_tokens(&self, user: &User) -> AppResult<LoginResponse> {
        use jsonwebtoken::{encode, Header, EncodingKey};
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize)]
        struct Claims {
            sub: String,
            exp: usize,
        }

        let auth_user = AuthUser {
            id: user.id,
            email: user.email.clone(),
            username: user.username.clone(),
            name: user.name.clone(),
            avatar_url: user.avatar_url.clone(),
        };

        // 生成访问令牌
        let access_claims = Claims {
            sub: user.id.to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::seconds(self.config.jwt_access_token_expires_in as i64)).timestamp() as usize,
        };

        let access_token = encode(
            &Header::default(),
            &access_claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_ref())
        )?;

        // 生成刷新令牌
        let refresh_claims = Claims {
            sub: user.id.to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::seconds(self.config.jwt_refresh_token_expires_in as i64)).timestamp() as usize,
        };

        let refresh_token = encode(
            &Header::default(),
            &refresh_claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_ref())
        )?;

        Ok(LoginResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.jwt_access_token_expires_in as i64,
            user: auth_user,
        })
    }
}