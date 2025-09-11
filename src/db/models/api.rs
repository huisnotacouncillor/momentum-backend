use serde::Serialize;

// 统一API响应结构
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub code: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ErrorDetail>>,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct ResponseMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
}

#[derive(Serialize)]
pub struct Pagination {
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
    pub has_next: bool,
    pub has_prev: bool,
}

#[derive(Serialize)]
pub struct ErrorDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    pub code: String,
    pub message: String,
}

// 便捷构造函数
impl<T> ApiResponse<T> {
    pub fn success(data: T, message: &str) -> Self {
        Self {
            success: true,
            code: 200,
            message: message.to_string(),
            data: Some(data),
            meta: None,
            errors: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn success_with_meta(data: T, message: &str, meta: ResponseMeta) -> Self {
        Self {
            success: true,
            code: 200,
            message: message.to_string(),
            data: Some(data),
            meta: Some(meta),
            errors: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn created(data: T, message: &str) -> Self {
        Self {
            success: true,
            code: 201,
            message: message.to_string(),
            data: Some(data),
            meta: None,
            errors: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn error(code: u16, message: &str, errors: Vec<ErrorDetail>) -> Self {
        Self {
            success: false,
            code,
            message: message.to_string(),
            data: None,
            meta: None,
            errors: Some(errors),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn validation_error(errors: Vec<ErrorDetail>) -> Self {
        Self {
            success: false,
            code: 400,
            message: "Validation failed".to_string(),
            data: None,
            meta: None,
            errors: Some(errors),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn unauthorized(message: &str) -> Self {
        Self {
            success: false,
            code: 401,
            message: message.to_string(),
            data: None,
            meta: None,
            errors: Some(vec![ErrorDetail {
                field: None,
                code: "UNAUTHORIZED".to_string(),
                message: message.to_string(),
            }]),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn forbidden(message: &str) -> Self {
        Self {
            success: false,
            code: 403,
            message: message.to_string(),
            data: None,
            meta: None,
            errors: Some(vec![ErrorDetail {
                field: None,
                code: "FORBIDDEN".to_string(),
                message: message.to_string(),
            }]),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn not_found(message: &str) -> Self {
        Self {
            success: false,
            code: 404,
            message: message.to_string(),
            data: None,
            meta: None,
            errors: Some(vec![ErrorDetail {
                field: None,
                code: "NOT_FOUND".to_string(),
                message: message.to_string(),
            }]),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn ok(message: &str) -> Self {
        Self {
            success: true,
            code: 200,
            message: message.to_string(),
            data: None,
            meta: None,
            errors: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn not_implemented(message: &str) -> Self {
        Self {
            success: false,
            code: 501,
            message: message.to_string(),
            data: None,
            meta: None,
            errors: Some(vec![ErrorDetail {
                field: None,
                code: "NOT_IMPLEMENTED".to_string(),
                message: message.to_string(),
            }]),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn conflict(message: &str, field: Option<String>, error_code: &str) -> Self {
        Self {
            success: false,
            code: 409,
            message: message.to_string(),
            data: None,
            meta: None,
            errors: Some(vec![ErrorDetail {
                field,
                code: error_code.to_string(),
                message: message.to_string(),
            }]),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn bad_request(message: &str) -> Self {
        Self {
            success: false,
            code: 400,
            message: message.to_string(),
            data: None,
            meta: None,
            errors: Some(vec![ErrorDetail {
                field: None,
                code: "BAD_REQUEST".to_string(),
                message: message.to_string(),
            }]),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn internal_error(message: &str) -> Self {
        Self {
            success: false,
            code: 500,
            message: message.to_string(),
            data: None,
            meta: None,
            errors: Some(vec![ErrorDetail {
                field: None,
                code: "INTERNAL_ERROR".to_string(),
                message: message.to_string(),
            }]),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// 业务错误码常量
pub mod error_codes {
    // 认证相关
    pub const AUTH_INVALID_EMAIL: &str = "AUTH_001";
    pub const AUTH_WEAK_PASSWORD: &str = "AUTH_002";
    pub const AUTH_USER_NOT_FOUND: &str = "AUTH_003";
    pub const AUTH_INVALID_PASSWORD: &str = "AUTH_004";
    pub const AUTH_ACCOUNT_DISABLED: &str = "AUTH_005";
    pub const AUTH_INVALID_TOKEN: &str = "AUTH_006";

    // 用户相关
    pub const USER_USERNAME_EXISTS: &str = "USER_001";
    pub const USER_EMAIL_EXISTS: &str = "USER_002";
    pub const USER_INCOMPLETE_PROFILE: &str = "USER_003";

    // 工作空间相关
    pub const WORKSPACE_NOT_FOUND: &str = "WORKSPACE_001";
    pub const WORKSPACE_ACCESS_DENIED: &str = "WORKSPACE_002";
    pub const WORKSPACE_NAME_EXISTS: &str = "WORKSPACE_003";

    // 团队相关
    pub const TEAM_NOT_FOUND: &str = "TEAM_001";
    pub const TEAM_NOT_MEMBER: &str = "TEAM_002";
    pub const TEAM_INSUFFICIENT_PERMISSIONS: &str = "TEAM_003";

    // 系统相关
    pub const SYSTEM_DATABASE_ERROR: &str = "SYSTEM_001";
    pub const SYSTEM_CACHE_ERROR: &str = "SYSTEM_002";
    pub const SYSTEM_EXTERNAL_SERVICE_ERROR: &str = "SYSTEM_003";
}
