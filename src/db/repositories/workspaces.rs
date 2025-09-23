use diesel::prelude::*;

use crate::db::models::workspace::{Workspace, NewWorkspace};

pub struct WorkspacesRepo;

impl WorkspacesRepo {
    pub fn insert(conn: &mut PgConnection, new_ws: &NewWorkspace) -> Result<Workspace, diesel::result::Error> {
        diesel::insert_into(crate::schema::workspaces::table)
            .values(new_ws)
            .get_result(conn)
    }

    pub fn exists_url_key(conn: &mut PgConnection, url: &str) -> Result<bool, diesel::result::Error> {
        use crate::schema::workspaces::dsl::*;
        diesel::select(diesel::dsl::exists(workspaces.filter(url_key.eq(url))))
            .get_result(conn)
    }

    pub fn find_by_id(conn: &mut PgConnection, workspace_id: uuid::Uuid) -> Result<Option<Workspace>, diesel::result::Error> {
        use crate::schema::workspaces::dsl::*;
        workspaces.filter(id.eq(workspace_id)).first::<Workspace>(conn).optional()
    }

    pub fn update_fields(
        conn: &mut PgConnection,
        workspace_id: uuid::Uuid,
        name: Option<&str>,
        url_key: Option<&str>,
        logo_url: Option<&str>,
    ) -> Result<Workspace, diesel::result::Error> {
        use crate::schema::workspaces::dsl as w;

        // Update each field individually
        if let Some(name_val) = name {
            diesel::update(w::workspaces.filter(w::id.eq(workspace_id)))
                .set(w::name.eq(name_val))
                .execute(conn)?;
        }
        if let Some(key) = url_key {
            diesel::update(w::workspaces.filter(w::id.eq(workspace_id)))
                .set(w::url_key.eq(key))
                .execute(conn)?;
        }
        if let Some(logo) = logo_url {
            diesel::update(w::workspaces.filter(w::id.eq(workspace_id)))
                .set(w::logo_url.eq(logo))
                .execute(conn)?;
        }

        // Return the updated workspace
        w::workspaces.filter(w::id.eq(workspace_id)).first::<Workspace>(conn)
    }

    pub fn delete_by_id(conn: &mut PgConnection, workspace_id: uuid::Uuid) -> Result<usize, diesel::result::Error> {
        use crate::schema::workspaces::dsl::*;
        diesel::delete(workspaces.filter(id.eq(workspace_id))).execute(conn)
    }
}


