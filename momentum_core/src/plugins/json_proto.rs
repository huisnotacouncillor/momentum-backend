//! JSON 工具（占位，P0-2 阶段扩展为完整的 prost Struct/Value 转换）

use serde_json::Value;

/// 把 serde_json::Value 转成紧凑字符串（用于 DB 存储 / gRPC 透传）
pub fn value_to_string(v: &Value) -> Result<String, serde_json::Error> {
    serde_json::to_string(v)
}

pub fn value_to_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null => None,
        _ => Some(v.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_to_text() {
        assert_eq!(
            value_to_text(&serde_json::json!("hello")),
            Some("hello".to_string())
        );
        assert_eq!(
            value_to_text(&serde_json::json!(42)),
            Some("42".to_string())
        );
        assert_eq!(
            value_to_text(&serde_json::json!(true)),
            Some("true".to_string())
        );
        assert_eq!(value_to_text(&serde_json::json!(null)), None);
    }
}
