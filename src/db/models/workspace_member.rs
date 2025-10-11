use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::Write;
use uuid::Uuid;

use crate::schema::workspace_members;

// WorkspaceMemberRole枚举定义
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, diesel::FromSqlRow, diesel::AsExpression,
)]
#[diesel(sql_type = crate::schema::sql_types::WorkspaceUserRole)]
pub enum WorkspaceMemberRole {
    Owner,
    Admin,
    Member,
    Guest,
}

impl diesel::serialize::ToSql<crate::schema::sql_types::WorkspaceUserRole, diesel::pg::Pg>
    for WorkspaceMemberRole
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        match *self {
            WorkspaceMemberRole::Owner => out.write_all(b"owner")?,
            WorkspaceMemberRole::Admin => out.write_all(b"admin")?,
            WorkspaceMemberRole::Member => out.write_all(b"member")?,
            WorkspaceMemberRole::Guest => out.write_all(b"guest")?,
        }
        Ok(diesel::serialize::IsNull::No)
    }
}

impl diesel::deserialize::FromSql<crate::schema::sql_types::WorkspaceUserRole, diesel::pg::Pg>
    for WorkspaceMemberRole
{
    fn from_sql(
        bytes: <diesel::pg::Pg as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        match <String as diesel::deserialize::FromSql<diesel::sql_types::Text, diesel::pg::Pg>>::from_sql(bytes)?.as_str() {
            "owner" => Ok(WorkspaceMemberRole::Owner),
            "admin" => Ok(WorkspaceMemberRole::Admin),
            "member" => Ok(WorkspaceMemberRole::Member),
            "guest" => Ok(WorkspaceMemberRole::Guest),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

// WorkspaceMember模型定义
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone, Debug)]
#[diesel(table_name = workspace_members)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct WorkspaceMember {
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub role: WorkspaceMemberRole,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = workspace_members)]
pub struct NewWorkspaceMember {
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub role: WorkspaceMemberRole,
}

#[derive(AsChangeset)]
#[diesel(table_name = workspace_members)]
pub struct UpdateWorkspaceMember {
    pub role: Option<WorkspaceMemberRole>,
}
