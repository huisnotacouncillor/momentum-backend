use diesel::prelude::*;

use crate::db::models::project_status::{ProjectStatus, NewProjectStatus, ProjectStatusCategory};

pub struct ProjectStatusRepo;

impl ProjectStatusRepo {
    pub fn list_by_workspace(conn: &mut PgConnection, ws_id: uuid::Uuid) -> Result<Vec<ProjectStatus>, diesel::result::Error> {
        use crate::schema::project_statuses::dsl::*;
        project_statuses
            .filter(workspace_id.eq(ws_id))
            .order(created_at.desc())
            .load::<ProjectStatus>(conn)
    }

    pub fn exists_by_name(conn: &mut PgConnection, ws_id: uuid::Uuid, status_name: &str) -> Result<bool, diesel::result::Error> {
        use crate::schema::project_statuses::dsl::*;
        diesel::select(diesel::dsl::exists(
            project_statuses.filter(workspace_id.eq(ws_id)).filter(name.eq(status_name))
        )).get_result(conn)
    }

    pub fn exists_by_name_excluding_id(
        conn: &mut PgConnection,
        ws_id: uuid::Uuid,
        status_name: &str,
        exclude_id: uuid::Uuid,
    ) -> Result<bool, diesel::result::Error> {
        use crate::schema::project_statuses::dsl::*;
        diesel::select(diesel::dsl::exists(
            project_statuses
                .filter(workspace_id.eq(ws_id))
                .filter(name.eq(status_name))
                .filter(id.ne(exclude_id))
        )).get_result(conn)
    }

    pub fn insert(conn: &mut PgConnection, new_status: &NewProjectStatus) -> Result<ProjectStatus, diesel::result::Error> {
        diesel::insert_into(crate::schema::project_statuses::table)
            .values(new_status)
            .get_result(conn)
    }

    pub fn find_by_id_in_workspace(conn: &mut PgConnection, ws_id: uuid::Uuid, status_id: uuid::Uuid) -> Result<Option<ProjectStatus>, diesel::result::Error> {
        use crate::schema::project_statuses::dsl::*;
        project_statuses
            .filter(id.eq(status_id))
            .filter(workspace_id.eq(ws_id))
            .first::<ProjectStatus>(conn)
            .optional()
    }

    pub fn update_fields(
        conn: &mut PgConnection,
        status_id_val: uuid::Uuid,
        new_name: Option<String>,
        new_description: Option<Option<String>>,
        new_color: Option<Option<String>>,
        new_category: Option<ProjectStatusCategory>,
    ) -> Result<ProjectStatus, diesel::result::Error> {
        use crate::schema::project_statuses::dsl as ps;
        // Build branch updates for type safety
        if let (Some(n), Some(desc), Some(col), Some(cat)) = (new_name.clone(), new_description.clone(), new_color.clone(), new_category.clone()) {
            return diesel::update(ps::project_statuses.filter(ps::id.eq(status_id_val)))
                .set((ps::name.eq(n), ps::description.eq(desc), ps::color.eq(col), ps::category.eq(cat)))
                .get_result(conn);
        }
        if let Some(n) = new_name.clone() {
            return diesel::update(ps::project_statuses.filter(ps::id.eq(status_id_val)))
                .set(ps::name.eq(n))
                .get_result(conn);
        }
        if let Some(desc) = new_description.clone() {
            return diesel::update(ps::project_statuses.filter(ps::id.eq(status_id_val)))
                .set(ps::description.eq(desc))
                .get_result(conn);
        }
        if let Some(col) = new_color.clone() {
            return diesel::update(ps::project_statuses.filter(ps::id.eq(status_id_val)))
                .set(ps::color.eq(col))
                .get_result(conn);
        }
        if let Some(cat) = new_category.clone() {
            return diesel::update(ps::project_statuses.filter(ps::id.eq(status_id_val)))
                .set(ps::category.eq(cat))
                .get_result(conn);
        }
        use crate::schema::project_statuses::dsl::*;
        project_statuses.filter(id.eq(status_id_val)).first::<ProjectStatus>(conn)
    }

    pub fn delete_by_id(conn: &mut PgConnection, status_id_val: uuid::Uuid) -> Result<usize, diesel::result::Error> {
        use crate::schema::project_statuses::dsl::*;
        diesel::delete(project_statuses.filter(id.eq(status_id_val))).execute(conn)
    }
}


