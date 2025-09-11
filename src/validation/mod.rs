use axum::{
    async_trait,
    extract::FromRequest,
    http::{Request, StatusCode},
    Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::{
    db::models::api::{ApiResponse, ErrorDetail},
    error::AppError,
};

/// 验证的 JSON 提取器
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S, axum::body::Body> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request<axum::body::Body>, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|_| AppError::Validation { message: "Invalid JSON format".to_string() })?;

        value.validate().map_err(|errors| {
            let error_details: Vec<ErrorDetail> = errors
                .field_errors()
                .iter()
                .flat_map(|(field, field_errors)| {
                    field_errors.iter().map(move |error| ErrorDetail {
                        field: Some(field.to_string()),
                        code: error.code.to_string(),
                        message: error.message.as_ref()
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| format!("Validation failed for field: {}", field)),
                    })
                })
                .collect();

            AppError::Validation { message: format!("Validation failed with {} errors", error_details.len()) }
        })?;

        Ok(ValidatedJson(value))
    }
}

/// 验证错误响应辅助函数
pub fn validation_error_response(errors: Vec<ErrorDetail>) -> (StatusCode, Json<ApiResponse<()>>) {
    let response = ApiResponse::validation_error(errors);
    (StatusCode::BAD_REQUEST, Json(response))
}

/// 常用验证规则
pub mod rules {
    use validator::ValidationError;

    /// 验证密码强度
    pub fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
        let mut score = 0;

        // 长度检查
        if password.len() >= 8 {
            score += 1;
        }

        // 包含小写字母
        if password.chars().any(|c| c.is_lowercase()) {
            score += 1;
        }

        // 包含大写字母
        if password.chars().any(|c| c.is_uppercase()) {
            score += 1;
        }

        // 包含数字
        if password.chars().any(|c| c.is_numeric()) {
            score += 1;
        }

        // 包含特殊字符
        if password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c)) {
            score += 1;
        }

        if score < 3 {
            return Err(ValidationError::new("weak_password"));
        }

        Ok(())
    }

    /// 验证用户名格式
    pub fn validate_username_format(username: &str) -> Result<(), ValidationError> {
        // 只允许字母、数字、下划线和连字符
        if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(ValidationError::new("invalid_username_format"));
        }

        // 不能以数字开头
        if username.chars().next().map_or(false, |c| c.is_numeric()) {
            return Err(ValidationError::new("username_starts_with_number"));
        }

        Ok(())
    }

    /// 验证工作空间URL键格式
    pub fn validate_workspace_url_key(url_key: &str) -> Result<(), ValidationError> {
        // 只允许小写字母、数字和连字符
        if !url_key.chars().all(|c| c.is_ascii_lowercase() || c.is_numeric() || c == '-') {
            return Err(ValidationError::new("invalid_url_key_format"));
        }

        // 不能以连字符开头或结尾
        if url_key.starts_with('-') || url_key.ends_with('-') {
            return Err(ValidationError::new("url_key_invalid_hyphens"));
        }

        Ok(())
    }
}