use crate::schema::sql_types::LabelLevelEnum;
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel::{AsExpression, FromSqlRow, Queryable};
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProjectStatus {
    Planned,
    Active,
    Paused,
    Completed,
    Canceled,
}

impl FromSql<Text, Pg> for ProjectStatus {
    fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let s = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        match s.as_str() {
            "planned" => Ok(ProjectStatus::Planned),
            "active" => Ok(ProjectStatus::Active),
            "paused" => Ok(ProjectStatus::Paused),
            "completed" => Ok(ProjectStatus::Completed),
            "canceled" => Ok(ProjectStatus::Canceled),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl ToSql<Text, Pg> for ProjectStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            ProjectStatus::Planned => out.write_all(b"planned")?,
            ProjectStatus::Active => out.write_all(b"active")?,
            ProjectStatus::Paused => out.write_all(b"paused")?,
            ProjectStatus::Completed => out.write_all(b"completed")?,
            ProjectStatus::Canceled => out.write_all(b"canceled")?,
        }
        Ok(IsNull::No)
    }
}

impl Queryable<Text, Pg> for ProjectStatus {
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CycleStatus {
    Planned,
    Active,
    Completed,
}

impl FromSql<Text, Pg> for CycleStatus {
    fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let s = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        match s.as_str() {
            "planned" => Ok(CycleStatus::Planned),
            "active" => Ok(CycleStatus::Active),
            "completed" => Ok(CycleStatus::Completed),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl ToSql<Text, Pg> for CycleStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            CycleStatus::Planned => out.write_all(b"planned")?,
            CycleStatus::Active => out.write_all(b"active")?,
            CycleStatus::Completed => out.write_all(b"completed")?,
        }
        Ok(IsNull::No)
    }
}

impl Queryable<Text, Pg> for CycleStatus {
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssuePriority {
    None,
    Low,
    Medium,
    High,
    Urgent,
}

impl FromSql<Text, Pg> for IssuePriority {
    fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let s = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        match s.as_str() {
            "none" => Ok(IssuePriority::None),
            "low" => Ok(IssuePriority::Low),
            "medium" => Ok(IssuePriority::Medium),
            "high" => Ok(IssuePriority::High),
            "urgent" => Ok(IssuePriority::Urgent),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl ToSql<Text, Pg> for IssuePriority {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            IssuePriority::None => out.write_all(b"none")?,
            IssuePriority::Low => out.write_all(b"low")?,
            IssuePriority::Medium => out.write_all(b"medium")?,
            IssuePriority::High => out.write_all(b"high")?,
            IssuePriority::Urgent => out.write_all(b"urgent")?,
        }
        Ok(IsNull::No)
    }
}

impl Queryable<Text, Pg> for IssuePriority {
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

/// Project priority enum, using the same values as IssuePriority
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
#[serde(rename_all = "lowercase")]
pub enum ProjectPriority {
    None,
    Low,
    Medium,
    High,
    Urgent,
}

impl FromSql<Text, Pg> for ProjectPriority {
    fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let s = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        match s.as_str() {
            "none" => Ok(ProjectPriority::None),
            "low" => Ok(ProjectPriority::Low),
            "medium" => Ok(ProjectPriority::Medium),
            "high" => Ok(ProjectPriority::High),
            "urgent" => Ok(ProjectPriority::Urgent),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl ToSql<Text, Pg> for ProjectPriority {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            ProjectPriority::None => out.write_all(b"none")?,
            ProjectPriority::Low => out.write_all(b"low")?,
            ProjectPriority::Medium => out.write_all(b"medium")?,
            ProjectPriority::High => out.write_all(b"high")?,
            ProjectPriority::Urgent => out.write_all(b"urgent")?,
        }
        Ok(IsNull::No)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, diesel::expression::AsExpression)]
#[diesel(sql_type = LabelLevelEnum)]
pub enum LabelLevel {
    Project,
    Issue,
}

impl FromSql<LabelLevelEnum, Pg> for LabelLevel {
    fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let s = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        match s.as_str() {
            "project" => Ok(LabelLevel::Project),
            "issue" => Ok(LabelLevel::Issue),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl ToSql<LabelLevelEnum, Pg> for LabelLevel {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            LabelLevel::Project => out.write_all(b"project")?,
            LabelLevel::Issue => out.write_all(b"issue")?,
        }
        Ok(IsNull::No)
    }
}

impl Queryable<LabelLevelEnum, Pg> for LabelLevel {
    type Row = Self;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}
