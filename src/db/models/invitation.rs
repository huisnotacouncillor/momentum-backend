use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::io::Write;

use crate::schema::{invitations};
use crate::db::models::workspace_member::WorkspaceMemberRole;

// InvitationStatus 枚举定义
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, diesel::FromSqlRow, diesel::AsExpression)]
#[diesel(sql_type = crate::schema::sql_types::InvitationStatus)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Cancelled,
}

impl diesel::serialize::ToSql<crate::schema::sql_types::InvitationStatus, diesel::pg::Pg> for InvitationStatus {
    fn to_sql<'b>(&'b self, out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        match *self {
            InvitationStatus::Pending => out.write_all(b"pending")?,
            InvitationStatus::Accepted => out.write_all(b"accepted")?,
            InvitationStatus::Declined => out.write_all(b"declined")?,
            InvitationStatus::Cancelled => out.write_all(b"cancelled")?,
        }
        Ok(diesel::serialize::IsNull::No)
    }
}

impl diesel::deserialize::FromSql<crate::schema::sql_types::InvitationStatus, diesel::pg::Pg> for InvitationStatus {
    fn from_sql(bytes: <diesel::pg::Pg as diesel::backend::Backend>::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        match <String as diesel::deserialize::FromSql<diesel::sql_types::Text, diesel::pg::Pg>>::from_sql(bytes)?.as_str() {
            "pending" => Ok(InvitationStatus::Pending),
            "accepted" => Ok(InvitationStatus::Accepted),
            "declined" => Ok(InvitationStatus::Declined),
            "cancelled" => Ok(InvitationStatus::Cancelled),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

// Invitation 模型定义
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone, Debug)]
#[diesel(table_name = invitations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Invitation {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub email: String,
    pub role: WorkspaceMemberRole,
    pub status: InvitationStatus,
    pub invited_by: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Insertable)]
#[diesel(table_name = invitations)]
pub struct NewInvitation {
    pub workspace_id: Uuid,
    pub email: String,
    pub role: WorkspaceMemberRole,
    pub invited_by: Uuid,
}

#[derive(AsChangeset)]
#[diesel(table_name = invitations)]
pub struct UpdateInvitation {
    pub status: Option<InvitationStatus>,
}