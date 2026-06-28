use diesel::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

use crate::db::models::issue_field_definition::IssueFieldDefinition;
use crate::db::models::issue_field_value::{
    IssueFieldValue, IssueFieldValueChangeset, NewIssueFieldValue,
};
use crate::schema::{issue_field_definitions, issue_field_values};

pub struct IssueFieldValueRepo;

impl IssueFieldValueRepo {
    /// 查单个 issue 的所有字段值，返回 {field_key: value}
    pub fn list_by_issue(
        conn: &mut PgConnection,
        issue_id: Uuid,
    ) -> Result<HashMap<String, serde_json::Value>, diesel::result::Error> {
        let rows: Vec<(String, serde_json::Value)> = issue_field_values::table
            .inner_join(
                issue_field_definitions::table
                    .on(issue_field_values::field_id.eq(issue_field_definitions::id)),
            )
            .filter(issue_field_values::issue_id.eq(issue_id))
            .select((
                issue_field_definitions::field_key,
                issue_field_values::value,
            ))
            .load::<(String, serde_json::Value)>(conn)?;
        Ok(rows.into_iter().collect())
    }

    /// 批量查多个 issue 的字段值
    pub fn list_by_issues(
        conn: &mut PgConnection,
        issue_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, HashMap<String, serde_json::Value>>, diesel::result::Error> {
        let rows: Vec<(Uuid, String, serde_json::Value)> = issue_field_values::table
            .inner_join(
                issue_field_definitions::table
                    .on(issue_field_values::field_id.eq(issue_field_definitions::id)),
            )
            .filter(issue_field_values::issue_id.eq_any(issue_ids))
            .select((
                issue_field_values::issue_id,
                issue_field_definitions::field_key,
                issue_field_values::value,
            ))
            .load::<(Uuid, String, serde_json::Value)>(conn)?;
        let mut result: HashMap<Uuid, HashMap<String, serde_json::Value>> = HashMap::new();
        for (issue_id, key, val) in rows {
            result.entry(issue_id).or_default().insert(key, val);
        }
        Ok(result)
    }

    pub fn upsert(
        conn: &mut PgConnection,
        issue_id: Uuid,
        field_id: Uuid,
        value: serde_json::Value,
        text_value: Option<String>,
    ) -> Result<IssueFieldValue, diesel::result::Error> {
        let new_value = NewIssueFieldValue {
            issue_id,
            field_id,
            value: value.clone(),
            text_value: text_value.clone(),
        };
        diesel::insert_into(issue_field_values::table)
            .values(&new_value)
            .on_conflict((issue_field_values::issue_id, issue_field_values::field_id))
            .do_update()
            .set(IssueFieldValueChangeset {
                value,
                text_value,
                updated_at: chrono::Utc::now(),
            })
            .get_result(conn)
    }

    pub fn delete_by_key(
        conn: &mut PgConnection,
        issue_id: Uuid,
        field_id: Uuid,
    ) -> Result<usize, diesel::result::Error> {
        diesel::delete(
            issue_field_values::table
                .filter(issue_field_values::issue_id.eq(issue_id))
                .filter(issue_field_values::field_id.eq(field_id)),
        )
        .execute(conn)
    }
}

/// 把 JSON value 转成 text_value（用于搜索/过滤）
pub fn value_to_text(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Null => None,
        _ => Some(v.to_string()),
    }
}

/// 读取字段定义（不直接关联 repos），给上层用
pub fn _typecheck() {
    let _: IssueFieldDefinition = IssueFieldDefinition {
        id: Uuid::nil(),
        workspace_id: Uuid::nil(),
        plugin_id: String::new(),
        field_key: String::new(),
        label: String::new(),
        field_type: String::new(),
        options: None,
        required: false,
        sort_order: 0,
        created_at: chrono::Utc::now(),
    };
}
