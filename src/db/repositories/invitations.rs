use diesel::prelude::*;

use crate::db::models::invitation::{Invitation, NewInvitation, InvitationStatus};

pub struct InvitationsRepo;

impl InvitationsRepo {
    pub fn insert(conn: &mut PgConnection, new_inv: &NewInvitation) -> Result<Invitation, diesel::result::Error> {
        diesel::insert_into(crate::schema::invitations::table)
            .values(new_inv)
            .get_result(conn)
    }

    pub fn pending_exists_for_email(conn: &mut PgConnection, ws_id: uuid::Uuid, email_val: &str) -> Result<bool, diesel::result::Error> {
        use crate::schema::invitations::dsl as inv;
        diesel::select(diesel::dsl::exists(
            inv::invitations
                .filter(inv::workspace_id.eq(ws_id))
                .filter(inv::email.eq(email_val))
                .filter(inv::status.eq(InvitationStatus::Pending))
                .filter(inv::expires_at.gt(chrono::Utc::now()))
        ))
        .get_result(conn)
    }

    pub fn update_status(conn: &mut PgConnection, inv_id: uuid::Uuid, status: InvitationStatus) -> Result<Invitation, diesel::result::Error> {
        use crate::schema::invitations::dsl as inv;
        diesel::update(inv::invitations.filter(inv::id.eq(inv_id)))
            .set((inv::status.eq(status), inv::updated_at.eq(chrono::Utc::now())))
            .get_result(conn)
    }

    pub fn list_by_workspace(conn: &mut PgConnection, ws: uuid::Uuid) -> Result<Vec<Invitation>, diesel::result::Error> {
        use crate::schema::invitations::dsl::*;
        invitations.filter(workspace_id.eq(ws)).order(created_at.desc()).load::<Invitation>(conn)
    }

    pub fn find_by_id(conn: &mut PgConnection, invitation_id: uuid::Uuid) -> Result<Option<Invitation>, diesel::result::Error> {
        use crate::schema::invitations::dsl::*;
        invitations.filter(id.eq(invitation_id)).first::<Invitation>(conn).optional()
    }

    pub fn delete_by_id(conn: &mut PgConnection, invitation_id: uuid::Uuid) -> Result<usize, diesel::result::Error> {
        use crate::schema::invitations::dsl::*;
        diesel::delete(invitations.filter(id.eq(invitation_id))).execute(conn)
    }
}


