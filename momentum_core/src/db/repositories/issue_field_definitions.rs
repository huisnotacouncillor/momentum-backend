use diesel::prelude::*;
use uuid::Uuid;

use crate::db::models::issue_field_definition::{IssueFieldDefinition, NewIssueFieldDefinition};
use crate::schema::issue_field_definitions;

pub struct IssueFieldDefinitionRepo;

impl IssueFieldDefinitionRepo {
    pub fn list_by_workspace(
        conn: &mut PgConnection,
        ws_id: Uuid,
    ) -> Result<Vec<IssueFieldDefinition>, diesel::result::Error> {
        issue_field_definitions::table
            .filter(issue_field_definitions::workspace_id.eq(ws_id))
            .order(issue_field_definitions::sort_order.asc())
            .load::<IssueFieldDefinition>(conn)
    }

    pub fn list_by_plugin(
        conn: &mut PgConnection,
        ws_id: Uuid,
        p_id: &str,
    ) -> Result<Vec<IssueFieldDefinition>, diesel::result::Error> {
        issue_field_definitions::table
            .filter(issue_field_definitions::workspace_id.eq(ws_id))
            .filter(issue_field_definitions::plugin_id.eq(p_id))
            .load::<IssueFieldDefinition>(conn)
    }

    pub fn find_by_key(
        conn: &mut PgConnection,
        ws_id: Uuid,
        p_id: &str,
        f_key: &str,
    ) -> Result<Option<IssueFieldDefinition>, diesel::result::Error> {
        issue_field_definitions::table
            .filter(issue_field_definitions::workspace_id.eq(ws_id))
            .filter(issue_field_definitions::plugin_id.eq(p_id))
            .filter(issue_field_definitions::field_key.eq(f_key))
            .first::<IssueFieldDefinition>(conn)
            .optional()
    }

    pub fn find_by_id(
        conn: &mut PgConnection,
        f_id: Uuid,
    ) -> Result<Option<IssueFieldDefinition>, diesel::result::Error> {
        issue_field_definitions::table
            .filter(issue_field_definitions::id.eq(f_id))
            .first::<IssueFieldDefinition>(conn)
            .optional()
    }

    pub fn upsert(
        conn: &mut PgConnection,
        new_def: &NewIssueFieldDefinition,
    ) -> Result<IssueFieldDefinition, diesel::result::Error> {
        diesel::insert_into(issue_field_definitions::table)
            .values(new_def)
            .on_conflict((
                issue_field_definitions::workspace_id,
                issue_field_definitions::plugin_id,
                issue_field_definitions::field_key,
            ))
            .do_update()
            .set((
                issue_field_definitions::label.eq(&new_def.label),
                issue_field_definitions::field_type.eq(&new_def.field_type),
                issue_field_definitions::options.eq(&new_def.options),
                issue_field_definitions::required.eq(&new_def.required),
                issue_field_definitions::sort_order.eq(&new_def.sort_order),
            ))
            .get_result(conn)
    }

    pub fn delete_by_plugin(
        conn: &mut PgConnection,
        ws_id: Uuid,
        p_id: &str,
    ) -> Result<usize, diesel::result::Error> {
        diesel::delete(
            issue_field_definitions::table
                .filter(issue_field_definitions::workspace_id.eq(ws_id))
                .filter(issue_field_definitions::plugin_id.eq(p_id)),
        )
        .execute(conn)
    }
}
