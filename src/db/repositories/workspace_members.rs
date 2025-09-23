use diesel::prelude::*;

use crate::db::models::workspace_member::{WorkspaceMember, NewWorkspaceMember};

pub struct WorkspaceMembersRepo;

impl WorkspaceMembersRepo {
    pub fn insert(conn: &mut PgConnection, new_member: &NewWorkspaceMember) -> Result<WorkspaceMember, diesel::result::Error> {
        diesel::insert_into(crate::schema::workspace_members::table)
            .values(new_member)
            .get_result(conn)
    }

    pub fn list_by_workspace(conn: &mut PgConnection, ws: uuid::Uuid) -> Result<Vec<WorkspaceMember>, diesel::result::Error> {
        use crate::schema::workspace_members::dsl::*;
        workspace_members.filter(workspace_id.eq(ws)).order(created_at.desc()).load::<WorkspaceMember>(conn)
    }

    pub fn find(conn: &mut PgConnection, ws_id: uuid::Uuid, user: uuid::Uuid) -> Result<Option<WorkspaceMember>, diesel::result::Error> {
        use crate::schema::workspace_members::dsl::*;
        workspace_members
            .filter(workspace_id.eq(ws_id))
            .filter(user_id.eq(user))
            .first::<WorkspaceMember>(conn)
            .optional()
    }

    pub fn delete(conn: &mut PgConnection, ws_id_val: uuid::Uuid, user_id_val: uuid::Uuid) -> Result<usize, diesel::result::Error> {
        use crate::schema::workspace_members::dsl::*;
        diesel::delete(workspace_members.filter(workspace_id.eq(ws_id_val)).filter(user_id.eq(user_id_val))).execute(conn)
    }
}


