use diesel::prelude::*;

use crate::db::models::project::{Project, NewProject};

pub struct ProjectsRepo;

impl ProjectsRepo {
    pub fn exists_key_in_workspace(conn: &mut PgConnection, ws: uuid::Uuid, key: &str) -> Result<bool, diesel::result::Error> {
        use crate::schema::projects::dsl::*;
        diesel::select(diesel::dsl::exists(projects.filter(workspace_id.eq(ws)).filter(project_key.eq(key)))).get_result(conn)
    }

    pub fn insert(conn: &mut PgConnection, new_project: &NewProject) -> Result<Project, diesel::result::Error> {
        diesel::insert_into(crate::schema::projects::table).values(new_project).get_result(conn)
    }

    pub fn find_by_id_in_workspace(conn: &mut PgConnection, ws: uuid::Uuid, project_id: uuid::Uuid) -> Result<Option<Project>, diesel::result::Error> {
        use crate::schema::projects::dsl::*;
        projects
            .filter(id.eq(project_id))
            .filter(workspace_id.eq(ws))
            .first::<Project>(conn)
            .optional()
    }

    pub fn update_fields(
        conn: &mut PgConnection,
        project_id: uuid::Uuid,
        name: Option<&str>,
        description: Option<&str>,
        project_key: Option<&str>,
        project_status_id: Option<uuid::Uuid>,
        target_date: Option<Option<chrono::NaiveDateTime>>,
        priority: Option<&crate::db::enums::ProjectPriority>,
        roadmap_id: Option<Option<uuid::Uuid>>,
    ) -> Result<Project, diesel::result::Error> {
        use crate::schema::projects::dsl as p;

        // Update each field individually
        if let Some(name_val) = name {
            diesel::update(p::projects.filter(p::id.eq(project_id)))
                .set(p::name.eq(name_val))
                .execute(conn)?;
        }
        if let Some(desc) = description {
            diesel::update(p::projects.filter(p::id.eq(project_id)))
                .set(p::description.eq(desc))
                .execute(conn)?;
        }
        if let Some(key) = project_key {
            diesel::update(p::projects.filter(p::id.eq(project_id)))
                .set(p::project_key.eq(key))
                .execute(conn)?;
        }
        if let Some(status_id) = project_status_id {
            diesel::update(p::projects.filter(p::id.eq(project_id)))
                .set(p::project_status_id.eq(status_id))
                .execute(conn)?;
        }
        if let Some(target) = target_date {
            diesel::update(p::projects.filter(p::id.eq(project_id)))
                .set(p::target_date.eq(target.map(|dt| dt.date())))
                .execute(conn)?;
        }
        if let Some(priority_val) = priority {
            diesel::update(p::projects.filter(p::id.eq(project_id)))
                .set(p::priority.eq(priority_val))
                .execute(conn)?;
        }
        if let Some(roadmap) = roadmap_id {
            diesel::update(p::projects.filter(p::id.eq(project_id)))
                .set(p::roadmap_id.eq(roadmap))
                .execute(conn)?;
        }

        // Return the updated project
        p::projects.filter(p::id.eq(project_id)).first::<Project>(conn)
    }

    pub fn delete_by_id(conn: &mut PgConnection, project_id: uuid::Uuid) -> Result<usize, diesel::result::Error> {
        use crate::schema::projects::dsl::*;
        diesel::delete(projects.filter(id.eq(project_id))).execute(conn)
    }
}


