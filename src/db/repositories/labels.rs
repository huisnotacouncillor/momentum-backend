use diesel::prelude::*;

use crate::db::models::label::{Label, NewLabel};

pub struct LabelRepo;

impl LabelRepo {
    pub fn exists_by_name(
        conn: &mut PgConnection,
        ws_id: uuid::Uuid,
        label_name: &str,
    ) -> Result<bool, diesel::result::Error> {
        use crate::schema::labels::dsl::*;
        diesel::select(diesel::dsl::exists(
            labels
                .filter(workspace_id.eq(ws_id))
                .filter(name.eq(label_name)),
        ))
        .get_result(conn)
    }

    pub fn exists_by_name_excluding_id(
        conn: &mut PgConnection,
        ws_id: uuid::Uuid,
        label_name: &str,
        exclude_id: uuid::Uuid,
    ) -> Result<bool, diesel::result::Error> {
        use crate::schema::labels::dsl::*;
        diesel::select(diesel::dsl::exists(
            labels
                .filter(workspace_id.eq(ws_id))
                .filter(name.eq(label_name))
                .filter(id.ne(exclude_id)),
        ))
        .get_result(conn)
    }

    pub fn insert(
        conn: &mut PgConnection,
        new_label: &NewLabel,
    ) -> Result<Label, diesel::result::Error> {
        diesel::insert_into(crate::schema::labels::table)
            .values(new_label)
            .get_result(conn)
    }

    pub fn list_by_workspace(
        conn: &mut PgConnection,
        ws_id: uuid::Uuid,
    ) -> Result<Vec<Label>, diesel::result::Error> {
        use crate::schema::labels::dsl::*;
        labels
            .filter(workspace_id.eq(ws_id))
            .order(created_at.desc())
            .load::<Label>(conn)
    }

    pub fn find_by_id_in_workspace(
        conn: &mut PgConnection,
        ws_id: uuid::Uuid,
        label_id: uuid::Uuid,
    ) -> Result<Option<Label>, diesel::result::Error> {
        use crate::schema::labels::dsl::*;
        labels
            .filter(id.eq(label_id))
            .filter(workspace_id.eq(ws_id))
            .first::<Label>(conn)
            .optional()
    }

    pub fn update_fields(
        conn: &mut PgConnection,
        label_id_val: uuid::Uuid,
        changes: (
            Option<String>,
            Option<String>,
            Option<crate::db::enums::LabelLevel>,
        ),
    ) -> Result<Label, diesel::result::Error> {
        use crate::schema::labels::dsl as l;
        // Apply sets in branches to satisfy type system
        if let (Some(n), Some(c), Some(lvl)) =
            (changes.0.clone(), changes.1.clone(), changes.2.clone())
        {
            return diesel::update(l::labels.filter(l::id.eq(label_id_val)))
                .set((l::name.eq(n), l::color.eq(c), l::level.eq(lvl)))
                .get_result(conn);
        }
        if let (Some(n), Some(c)) = (changes.0.clone(), changes.1.clone()) {
            return diesel::update(l::labels.filter(l::id.eq(label_id_val)))
                .set((l::name.eq(n), l::color.eq(c)))
                .get_result(conn);
        }
        if let (Some(n), Some(lvl)) = (changes.0.clone(), changes.2.clone()) {
            return diesel::update(l::labels.filter(l::id.eq(label_id_val)))
                .set((l::name.eq(n), l::level.eq(lvl)))
                .get_result(conn);
        }
        if let (Some(c), Some(lvl)) = (changes.1.clone(), changes.2.clone()) {
            return diesel::update(l::labels.filter(l::id.eq(label_id_val)))
                .set((l::color.eq(c), l::level.eq(lvl)))
                .get_result(conn);
        }
        if let Some(n) = changes.0.clone() {
            return diesel::update(l::labels.filter(l::id.eq(label_id_val)))
                .set(l::name.eq(n))
                .get_result(conn);
        }
        if let Some(c) = changes.1.clone() {
            return diesel::update(l::labels.filter(l::id.eq(label_id_val)))
                .set(l::color.eq(c))
                .get_result(conn);
        }
        if let Some(lvl) = changes.2.clone() {
            return diesel::update(l::labels.filter(l::id.eq(label_id_val)))
                .set(l::level.eq(lvl))
                .get_result(conn);
        }
        // no changes provided; just return current row
        use crate::schema::labels::dsl::*;
        labels.filter(id.eq(label_id_val)).first::<Label>(conn)
    }

    pub fn delete_by_id(
        conn: &mut PgConnection,
        label_id_val: uuid::Uuid,
    ) -> Result<usize, diesel::result::Error> {
        use crate::schema::labels::dsl::*;
        diesel::delete(labels.filter(id.eq(label_id_val))).execute(conn)
    }
}
