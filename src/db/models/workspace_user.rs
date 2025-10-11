use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::Write;
use uuid::Uuid;

use crate::schema::workspace_members;

// WorkspaceUserRole枚举定义
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, diesel::FromSqlRow, diesel::AsExpression,
)]
#[diesel(sql_type = crate::schema::sql_types::WorkspaceUserRole)]
pub enum WorkspaceUserRole {
    Owner,
    Admin,
    Member,
    Guest,
}

impl diesel::serialize::ToSql<crate::schema::sql_types::WorkspaceUserRole, diesel::pg::Pg>
    for WorkspaceUserRole
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        match *self {
            WorkspaceUserRole::Owner => out.write_all(b"owner")?,
            WorkspaceUserRole::Admin => out.write_all(b"admin")?,
            WorkspaceUserRole::Member => out.write_all(b"member")?,
            WorkspaceUserRole::Guest => out.write_all(b"guest")?,
        }
        Ok(diesel::serialize::IsNull::No)
    }
}

impl diesel::deserialize::FromSql<crate::schema::sql_types::WorkspaceUserRole, diesel::pg::Pg>
    for WorkspaceUserRole
{
    fn from_sql(
        bytes: <diesel::pg::Pg as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        match <String as diesel::deserialize::FromSql<diesel::sql_types::Text, diesel::pg::Pg>>::from_sql(bytes)?.as_str() {
            "owner" => Ok(WorkspaceUserRole::Owner),
            "admin" => Ok(WorkspaceUserRole::Admin),
            "member" => Ok(WorkspaceUserRole::Member),
            "guest" => Ok(WorkspaceUserRole::Guest),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

// WorkspaceUser模型定义
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone, Debug)]
#[diesel(table_name = workspace_members)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct WorkspaceUser {
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub role: WorkspaceUserRole,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = workspace_members)]
pub struct NewWorkspaceUser {
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub role: WorkspaceUserRole,
}

#[derive(AsChangeset)]
#[diesel(table_name = workspace_members)]
pub struct UpdateWorkspaceUser {
    pub role: Option<WorkspaceUserRole>,
}
