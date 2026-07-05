use bcrypt::{hash, verify};
use diesel::prelude::*;
use uuid::Uuid;

use crate::config::Config;
use crate::utils::AssetUrlHelper;
use crate::{
    db::models::auth::{
        AuthUser, LoginRequest, LoginResponse, NewUser, NewUserCredential, RegisterRequest, User,
        UserProfile,
    },
    db::models::{
        project_status::{NewProjectStatus, ProjectStatusCategory},
        team::TeamInfo,
        workflow::{NewWorkflow, NewWorkflowState, WorkflowStateCategory},
        workspace::WorkspaceInfo,
    },
    db::models::workspace_member::WorkspaceMemberRole,
    db::repositories::auth::AuthRepo,
    db::repositories::workspaces::WorkspacesRepo,
    error::AppError,
    services::context::RequestContext,
    services::jwt::JwtService,
    validation::auth::{
        UpdateProfileChanges, validate_login_request, validate_register_request,
        validate_update_profile,
    },
};

pub struct AuthService;

impl AuthService {
    pub fn register(
        conn: &mut PgConnection,
        req: &RegisterRequest,
        asset_helper: &AssetUrlHelper,
    ) -> Result<LoginResponse, AppError> {
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

        // Hash password
        let hashed_password = hash(&req.password, bcrypt::DEFAULT_COST)
            .map_err(|_| AppError::internal("Failed to hash password"))?;

        let new_user = NewUser {
            email: req.email.clone(),
            username: req.username.clone(),
            name: req.name.clone(),
            avatar_url: None,
        };

        let user = conn.transaction::<User, AppError, _>(|conn| {
            let user = AuthRepo::insert_user(conn, &new_user)?;

            // Create credential
            let new_credential = NewUserCredential {
                user_id: user.id,
                credential_type: "password".to_string(),
                credential_hash: Some(hashed_password),
                oauth_provider_id: None,
                oauth_user_id: None,
                is_primary: true,
            };

            AuthRepo::insert_credential(conn, &new_credential)?;

            // Create default workspace: "{name}'s Workspace"
            let workspace_name = format!("{}'s Workspace", req.name);
            let workspace_url_key = format!("{}-workspace", req.username.to_lowercase());

            let new_workspace = crate::db::models::workspace::NewWorkspace {
                name: workspace_name,
                url_key: workspace_url_key,
                logo_url: None,
            };
            let workspace = WorkspacesRepo::insert(conn, &new_workspace)?;

            // Add user as Owner of the workspace
            let new_member = crate::db::models::workspace_member::NewWorkspaceMember {
                user_id: user.id,
                workspace_id: workspace.id,
                role: WorkspaceMemberRole::Owner,
            };
            use crate::schema::workspace_members::dsl::*;
            diesel::insert_into(workspace_members)
                .values(&new_member)
                .execute(conn)
                .map_err(|e| AppError::internal(format!("Failed to add workspace member: {}", e)))?;

            // Update user's current_workspace_id
            let user = AuthRepo::update_current_workspace(conn, user.id, workspace.id)?;

            // Create default team: "{username}'s Team"
            let team_key = format!("{}-team", req.username.to_lowercase())[..10].to_string();
            let new_team = crate::db::models::team::NewTeam {
                workspace_id: workspace.id,
                name: format!("{}'s Team", req.username),
                team_key,
                description: None,
                icon_url: None,
                is_private: false,
            };
            let team: crate::db::models::team::Team = diesel::insert_into(crate::schema::teams::table)
                .values(&new_team)
                .get_result(conn)
                .map_err(|e| AppError::internal(format!("Failed to create team: {}", e)))?;

            // Add user as admin of the team
            let new_team_member = crate::db::models::team::NewTeamMember {
                user_id: user.id,
                team_id: team.id,
                role: "admin".to_string(),
            };
            diesel::insert_into(crate::schema::team_members::table)
                .values(&new_team_member)
                .execute(conn)
                .map_err(|e| AppError::internal(format!("Failed to add team member: {}", e)))?;

            // Create default workflow with states for the team
            let new_workflow = NewWorkflow {
                name: "Default Workflow".to_string(),
                description: Some("Default workflow for new teams".to_string()),
                team_id: team.id,
                is_default: true,
            };
            let workflow: crate::db::models::workflow::Workflow =
                diesel::insert_into(crate::schema::workflows::table)
                    .values(&new_workflow)
                    .get_result(conn)
                    .map_err(|e| AppError::internal(format!("Failed to create workflow: {}", e)))?;

            // Create default workflow states
            let default_states = vec![
                (1, "Backlog", "#999999", WorkflowStateCategory::Backlog, true),
                (2, "Todo", "#999999", WorkflowStateCategory::Unstarted, false),
                (3, "In Progress", "#F1BF00", WorkflowStateCategory::Started, false),
                (4, "In Review", "#82E0AA", WorkflowStateCategory::Started, false),
                (5, "Done", "#0082FF", WorkflowStateCategory::Completed, false),
                (6, "Canceled", "#333333", WorkflowStateCategory::Canceled, false),
                (7, "Duplicated", "#333333", WorkflowStateCategory::Canceled, false),
            ];
            for (pos, name, color, category, is_default) in default_states {
                let state = NewWorkflowState {
                    workflow_id: workflow.id,
                    name: name.to_string(),
                    description: None,
                    color: Some(color.to_string()),
                    category,
                    position: pos,
                    is_default,
                };
                diesel::insert_into(crate::schema::workflow_states::table)
                    .values(&state)
                    .execute(conn)
                    .map_err(|e| AppError::internal(format!("Failed to create workflow state: {}", e)))?;
            }

            // Create default project status for the workspace
            let new_project_status = NewProjectStatus {
                name: "Planned".to_string(),
                description: Some("Default project status".to_string()),
                color: Some("#4A90D9".to_string()),
                category: ProjectStatusCategory::Planned,
                workspace_id: workspace.id,
            };
            let project_status: crate::db::models::project_status::ProjectStatus =
                diesel::insert_into(crate::schema::project_statuses::table)
                    .values(&new_project_status)
                    .get_result(conn)
                    .map_err(|e| AppError::internal(format!("Failed to create project status: {}", e)))?;

            // Create default project
            let project_key = format!("{}-1", req.username.to_lowercase())[..10].to_string();
            let new_project = crate::db::models::project::NewProject {
                workspace_id: workspace.id,
                roadmap_id: None,
                owner_id: user.id,
                name: "My First Project".to_string(),
                project_key,
                description: Some("Welcome to your first project!".to_string()),
                target_date: None,
                project_status_id: project_status.id,
                priority: None,
            };
            diesel::insert_into(crate::schema::projects::table)
                .values(&new_project)
                .execute(conn)
                .map_err(|e| AppError::internal(format!("Failed to create project: {}", e)))?;

            Ok(user)
        })?;

        // Generate JWT tokens
        let jwt_service = JwtService::from_config(&Config::from_env()?);

        let auth_user = AuthUser {
            id: user.id,
            email: user.email.clone(),
            username: user.username.clone(),
            name: user.name.clone(),
            avatar_url: user.get_processed_avatar_url(asset_helper),
        };

        let access_token = jwt_service
            .generate_access_token(&auth_user)
            .map_err(|_| AppError::internal("Failed to generate access token"))?;

        let refresh_token = jwt_service
            .generate_refresh_token(user.id)
            .map_err(|_| AppError::internal("Failed to generate refresh token"))?;

        // 获取当前工作空间的 url_key (注册时通常没有工作空间)
        let current_workspace_url_key = if let Some(workspace_id) = user.current_workspace_id {
            use crate::schema::workspaces;
            workspaces::table
                .filter(workspaces::id.eq(workspace_id))
                .select(workspaces::url_key)
                .first::<String>(conn)
                .optional()
                .map_err(|_| AppError::internal("Failed to get workspace url_key"))?
        } else {
            None
        };

        Ok(LoginResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            user: auth_user,
            current_workspace_url_key,
        })
    }

    pub fn login(
        conn: &mut PgConnection,
        req: &LoginRequest,
        asset_helper: &AssetUrlHelper,
    ) -> Result<LoginResponse, AppError> {
        validate_login_request(&req.email, &req.password)?;

        let user = AuthRepo::find_by_email(conn, &req.email)?
            .ok_or_else(|| AppError::auth("Invalid email or password"))?;

        let credential = AuthRepo::find_credential_by_user_id(conn, user.id)?
            .ok_or_else(|| AppError::auth("Invalid email or password"))?;

        let is_valid = verify(&req.password, credential.credential_hash.as_ref().unwrap())
            .map_err(|_| AppError::internal("Failed to verify password"))?;

        if !is_valid {
            return Err(AppError::auth("Invalid email or password"));
        }

        // Generate JWT tokens using the proper JWT service
        let jwt_service = JwtService::from_config(&Config::from_env()?);

        let auth_user = AuthUser {
            id: user.id,
            email: user.email.clone(),
            username: user.username.clone(),
            name: user.name.clone(),
            avatar_url: user.get_processed_avatar_url(asset_helper),
        };

        let access_token = jwt_service
            .generate_access_token(&auth_user)
            .map_err(|_| AppError::internal("Failed to generate access token"))?;

        let refresh_token = jwt_service
            .generate_refresh_token(user.id)
            .map_err(|_| AppError::internal("Failed to generate refresh token"))?;

        // 获取当前工作空间的 url_key
        let current_workspace_url_key = if let Some(workspace_id) = user.current_workspace_id {
            use crate::schema::workspaces;
            workspaces::table
                .filter(workspaces::id.eq(workspace_id))
                .select(workspaces::url_key)
                .first::<String>(conn)
                .optional()
                .map_err(|_| AppError::internal("Failed to get workspace url_key"))?
        } else {
            None
        };

        Ok(LoginResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            user: auth_user,
            current_workspace_url_key,
        })
    }

    pub fn get_profile(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        asset_helper: &AssetUrlHelper,
    ) -> Result<UserProfile, AppError> {
        let user =
            AuthRepo::find_by_id(conn, ctx.user_id)?.ok_or_else(|| AppError::not_found("user"))?;

        // Get user workspaces
        let workspaces = Self::get_user_workspaces(conn, ctx.user_id, asset_helper)?;

        // Get user teams
        let teams = Self::get_user_teams(conn, ctx.user_id)?;

        let processed_avatar_url = user.get_processed_avatar_url(asset_helper);
        Ok(UserProfile {
            id: user.id,
            name: user.name,
            username: user.username,
            email: user.email,
            avatar_url: processed_avatar_url,
            current_workspace_id: user.current_workspace_id,
            workspaces,
            teams,
        })
    }

    pub fn update_profile(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        changes: &crate::services::auth::types::UpdateProfileRequest,
        asset_helper: &AssetUrlHelper,
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

        let processed_avatar_url = updated_user.get_processed_avatar_url(asset_helper);
        Ok(UserProfile {
            id: updated_user.id,
            name: updated_user.name,
            username: updated_user.username,
            email: updated_user.email,
            avatar_url: processed_avatar_url,
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
        // P0 修复：验证用户是该工作区的成员，防止跨工作区越权
        let membership = crate::db::repositories::workspace_members::WorkspaceMembersRepo::find(
            conn,
            workspace_id,
            ctx.user_id,
        )
        .map_err(AppError::Database)?;

        if membership.is_none() {
            return Err(AppError::Forbidden {
                message: "User is not a member of this workspace".to_string(),
            });
        }

        let updated_user = AuthRepo::update_current_workspace(conn, ctx.user_id, workspace_id)?;
        Ok(updated_user)
    }

    /// Get all workspaces that the user has access to
    fn get_user_workspaces(
        conn: &mut PgConnection,
        user_id: Uuid,
        asset_helper: &AssetUrlHelper,
    ) -> Result<Vec<WorkspaceInfo>, AppError> {
        use crate::schema::{workspace_members, workspaces};

        let results = workspace_members::table
            .inner_join(workspaces::table.on(workspace_members::workspace_id.eq(workspaces::id)))
            .filter(workspace_members::user_id.eq(user_id))
            .select((
                workspaces::id,
                workspaces::name,
                workspaces::url_key,
                workspaces::logo_url,
            ))
            .load::<(Uuid, String, String, Option<String>)>(conn)
            .map_err(|_| AppError::internal("Failed to retrieve user workspaces"))?;

        Ok(results
            .into_iter()
            .map(|(id, name, url_key, logo_url)| {
                let processed_logo_url = logo_url.map(|url| asset_helper.process_url(&url));
                WorkspaceInfo {
                    id,
                    name,
                    url_key,
                    logo_url: processed_logo_url,
                }
            })
            .collect())
    }

    /// Get all teams that the user belongs to
    fn get_user_teams(conn: &mut PgConnection, user_id: Uuid) -> Result<Vec<TeamInfo>, AppError> {
        use crate::schema::{team_members, teams};

        let results = team_members::table
            .inner_join(teams::table.on(team_members::team_id.eq(teams::id)))
            .filter(team_members::user_id.eq(user_id))
            .select((
                teams::id,
                teams::name,
                teams::team_key,
                teams::description,
                teams::icon_url,
                teams::is_private,
                team_members::role,
            ))
            .load::<(
                Uuid,
                String,
                String,
                Option<String>,
                Option<String>,
                bool,
                String,
            )>(conn)
            .map_err(|_| AppError::internal("Failed to retrieve user teams"))?;

        Ok(results
            .into_iter()
            .map(
                |(id, name, team_key, description, icon_url, is_private, role)| TeamInfo {
                    id,
                    name,
                    team_key,
                    description,
                    icon_url,
                    is_private,
                    role,
                },
            )
            .collect())
    }

    /// Logout user - invalidate all active sessions
    pub fn logout(conn: &mut PgConnection, ctx: &RequestContext) -> Result<(), AppError> {
        use crate::schema::user_sessions::dsl::*;

        // Set all active sessions for this user to inactive
        diesel::update(user_sessions.filter(user_id.eq(ctx.user_id).and(is_active.eq(true))))
            .set(is_active.eq(false))
            .execute(conn)
            .map_err(|_| AppError::internal("Failed to logout user"))?;

        Ok(())
    }
}
