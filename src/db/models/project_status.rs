use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::serialize::{self, Output, ToSql};
use serde::{Deserialize, Serialize};
use std::io::Write;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum ProjectStatusCategory {
    Backlog,
    Planned,
    InProgress,
    Completed,
    Canceled,
}

impl ProjectStatusCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectStatusCategory::Backlog => "backlog",
            ProjectStatusCategory::Planned => "planned",
            ProjectStatusCategory::InProgress => "in_progress",
            ProjectStatusCategory::Completed => "completed",
            ProjectStatusCategory::Canceled => "canceled",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "backlog" => ProjectStatusCategory::Backlog,
            "planned" => ProjectStatusCategory::Planned,
            "in_progress" => ProjectStatusCategory::InProgress,
            "completed" => ProjectStatusCategory::Completed,
            "canceled" => ProjectStatusCategory::Canceled,
            _ => ProjectStatusCategory::Backlog,
        }
    }
}

impl std::fmt::Display for ProjectStatusCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ProjectStatusCategory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(ProjectStatusCategory::from_str(&s))
    }
}

impl serde::Serialize for ProjectStatusCategory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl ToSql<diesel::sql_types::Text, Pg> for ProjectStatusCategory {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        out.write_all(self.as_str().as_bytes())
            .map(|_| serialize::IsNull::No)
            .map_err(Into::into)
    }
}

impl FromSql<diesel::sql_types::Text, Pg> for ProjectStatusCategory {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        match std::str::from_utf8(bytes.as_bytes())? {
            "backlog" => Ok(ProjectStatusCategory::Backlog),
            "planned" => Ok(ProjectStatusCategory::Planned),
            "in_progress" => Ok(ProjectStatusCategory::InProgress),
            "completed" => Ok(ProjectStatusCategory::Completed),
            "canceled" => Ok(ProjectStatusCategory::Canceled),
            _ => Ok(ProjectStatusCategory::Backlog),
        }
    }
}

// ProjectStatus models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone, Debug)]
#[diesel(table_name = crate::schema::project_statuses)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ProjectStatus {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    #[serde(deserialize_with = "deserialize_category")]
    #[serde(serialize_with = "serialize_category")]
    pub category: ProjectStatusCategory,
    pub workspace_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn deserialize_category<'de, D>(deserializer: D) -> Result<ProjectStatusCategory, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(ProjectStatusCategory::from_str(&s))
}

fn serialize_category<S>(category: &ProjectStatusCategory, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(category.as_str())
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::project_statuses)]
pub struct NewProjectStatus {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    #[diesel(column_name = category)]
    pub category: ProjectStatusCategory,
    pub workspace_id: Uuid,
}

#[derive(Deserialize, Serialize)]
pub struct CreateProjectStatusRequest {
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub category: ProjectStatusCategory,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectStatusInfo {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    #[serde(serialize_with = "serialize_category")]
    #[serde(deserialize_with = "deserialize_category")]
    pub category: ProjectStatusCategory,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
