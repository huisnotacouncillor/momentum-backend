use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Label models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::labels)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Label {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::labels)]
pub struct NewLabel {
    pub workspace_id: Uuid,
    pub name: String,
    pub color: Option<String>,
}
