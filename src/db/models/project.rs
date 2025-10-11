use crate::db::enums::{ProjectPriority, ProjectStatus};
use crate::db::models::auth::UserBasicInfo;
use crate::db::models::project_status::ProjectStatusInfo;
use chrono;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

// 为ProjectPriority实现FromStr trait
impl FromStr for ProjectPriority {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(ProjectPriority::None),
            "low" => Ok(ProjectPriority::Low),
            "medium" => Ok(ProjectPriority::Medium),
            "high" => Ok(ProjectPriority::High),
            "urgent" => Ok(ProjectPriority::Urgent),
            _ => Err(()),
        }
    }
}

// 为ProjectPriority实现Display trait
impl std::fmt::Display for ProjectPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ProjectPriority::None => "none",
            ProjectPriority::Low => "low",
            ProjectPriority::Medium => "medium",
            ProjectPriority::High => "high",
            ProjectPriority::Urgent => "urgent",
        };
        write!(f, "{}", s)
    }
}

// Project models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::projects)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Project {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub roadmap_id: Option<Uuid>,
    pub owner_id: Uuid,
    pub name: String,
    pub project_key: String,
    pub description: Option<String>,
    pub target_date: Option<chrono::NaiveDate>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub project_status_id: Uuid,
    #[serde(
        serialize_with = "serialize_priority",
        deserialize_with = "deserialize_priority"
    )]
    pub priority: ProjectPriority,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::projects)]
pub struct NewProject {
    pub workspace_id: Uuid,
    pub roadmap_id: Option<Uuid>,
    pub owner_id: Uuid,
    pub name: String,
    pub project_key: String,
    pub description: Option<String>,
    pub target_date: Option<chrono::NaiveDate>,
    pub project_status_id: Uuid,
    pub priority: Option<ProjectPriority>,
}

#[derive(Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub roadmap_id: Option<Option<Uuid>>,
    pub target_date: Option<Option<chrono::NaiveDate>>,
    pub project_status_id: Option<Uuid>,
    #[serde(default, deserialize_with = "deserialize_optional_priority")]
    pub priority: Option<ProjectPriority>,
}

// Project API DTOs
#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub project_key: String,
    pub description: Option<String>,
    pub roadmap_id: Option<Uuid>,
    pub target_date: Option<chrono::NaiveDate>,
    pub project_status_id: Option<Uuid>,
    #[serde(default, deserialize_with = "deserialize_optional_priority")]
    pub priority: Option<ProjectPriority>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ProjectInfo {
    pub id: Uuid,
    pub name: String,
    pub project_key: String,
    pub description: Option<String>,
    pub status: ProjectStatusInfo,
    pub available_statuses: Vec<ProjectStatusInfo>, // 添加所有可用的项目状态
    pub owner: UserBasicInfo,
    pub target_date: Option<chrono::NaiveDate>,
    #[serde(
        serialize_with = "serialize_priority",
        deserialize_with = "deserialize_priority"
    )]
    pub priority: ProjectPriority,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn serialize_priority<S>(priority: &ProjectPriority, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let priority_str = match priority {
        ProjectPriority::None => "none",
        ProjectPriority::Low => "low",
        ProjectPriority::Medium => "medium",
        ProjectPriority::High => "high",
        ProjectPriority::Urgent => "urgent",
    };
    serializer.serialize_str(priority_str)
}

fn deserialize_priority<'de, D>(deserializer: D) -> Result<ProjectPriority, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
        "none" => Ok(ProjectPriority::None),
        "low" => Ok(ProjectPriority::Low),
        "medium" => Ok(ProjectPriority::Medium),
        "high" => Ok(ProjectPriority::High),
        "urgent" => Ok(ProjectPriority::Urgent),
        _ => Err(serde::de::Error::custom("Invalid project priority")),
    }
}

fn deserialize_optional_priority<'de, D>(
    deserializer: D,
) -> Result<Option<ProjectPriority>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let maybe_s = Option::<String>::deserialize(deserializer)?;
    match maybe_s {
        None => Ok(None),
        Some(s) => match s.as_str() {
            "none" => Ok(Some(ProjectPriority::None)),
            "low" => Ok(Some(ProjectPriority::Low)),
            "medium" => Ok(Some(ProjectPriority::Medium)),
            "high" => Ok(Some(ProjectPriority::High)),
            "urgent" => Ok(Some(ProjectPriority::Urgent)),
            _ => Err(serde::de::Error::custom("Invalid project priority")),
        },
    }
}

#[derive(Serialize, Debug)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectInfo>,
    pub total_count: i64,
}

#[derive(Deserialize)]
pub struct ProjectListQuery {
    pub workspace_id: Option<Uuid>,
    pub status: Option<ProjectStatus>,
    pub priority: Option<ProjectPriority>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}
