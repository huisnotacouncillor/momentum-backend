//! WebSocket 功能演示
//!
//! 这个演示展示了如何使用 WebSocket API 的核心功能：
//! 1. WebSocket 命令系统（CRUD 操作）
//! 2. 消息签名和安全
//! 3. 错误处理

use uuid::Uuid;

use rust_backend::db::enums::LabelLevel;
use rust_backend::websocket::{
    // 认证
    AuthenticatedUser,
    // 命令类型
    WebSocketCommand,
    // 错误处理
    WebSocketErrorCode,
    WebSocketErrorHandler,
    commands::types::{
        CreateLabelCommand, CreateTeamCommand, CreateWorkspaceCommand, UpdateLabelCommand,
        UpdateTeamCommand,
    },
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 WebSocket 功能演示");
    println!("================================\n");

    // 1. 创建测试用户
    println!("1️⃣ 创建测试用户");
    let test_user = AuthenticatedUser {
        user_id: Uuid::new_v4(),
        username: "demo_user".to_string(),
        email: "demo@example.com".to_string(),
        name: "Demo User".to_string(),
        avatar_url: None,
        current_workspace_id: Some(Uuid::new_v4()),
    };
    println!("   ✅ 用户ID: {}", test_user.user_id);
    println!("   ✅ 用户名: {}", test_user.username);
    println!("   ✅ 工作空间: {:?}\n", test_user.current_workspace_id);

    // 2. 演示命令系统
    println!("2️⃣ WebSocket 命令系统演示");
    demonstrate_commands()?;

    // 3. 演示错误处理
    println!("\n3️⃣ 错误处理演示");
    demonstrate_error_handling()?;

    println!("\n✅ 演示完成!");
    Ok(())
}

/// 演示 WebSocket 命令系统
fn demonstrate_commands() -> Result<(), Box<dyn std::error::Error>> {
    // Label 命令
    println!("   📌 Label 命令:");

    let create_label = WebSocketCommand::CreateLabel {
        data: CreateLabelCommand {
            name: "重要".to_string(),
            color: "#FF0000".to_string(),
            level: LabelLevel::Project,
        },
        request_id: Some("req-001".to_string()),
    };
    println!("      ✓ 创建标签命令已创建");

    let _update_label = WebSocketCommand::UpdateLabel {
        label_id: Uuid::new_v4(),
        data: UpdateLabelCommand {
            name: Some("紧急".to_string()),
            color: Some("#FF6600".to_string()),
            level: None,
        },
        request_id: Some("req-002".to_string()),
    };
    println!("      ✓ 更新标签命令已创建");

    let _delete_label = WebSocketCommand::DeleteLabel {
        label_id: Uuid::new_v4(),
        request_id: Some("req-003".to_string()),
    };
    println!("      ✓ 删除标签命令已创建");

    // Team 命令
    println!("\n   👥 Team 命令:");

    let _create_team = WebSocketCommand::CreateTeam {
        data: CreateTeamCommand {
            name: "开发团队".to_string(),
            team_key: "dev-team".to_string(),
            description: Some("后端开发团队".to_string()),
            icon_url: Some("https://example.com/icon.png".to_string()),
            is_private: false,
        },
        request_id: Some("req-004".to_string()),
    };
    println!("      ✓ 创建团队命令已创建");

    let _update_team = WebSocketCommand::UpdateTeam {
        team_id: Uuid::new_v4(),
        data: UpdateTeamCommand {
            name: Some("全栈团队".to_string()),
            team_key: Some("fullstack-team".to_string()),
            description: Some("全栈开发团队".to_string()),
            icon_url: None,
            is_private: None,
        },
        request_id: Some("req-005".to_string()),
    };
    println!("      ✓ 更新团队命令已创建");

    // Workspace 命令
    println!("\n   🏢 Workspace 命令:");

    let _create_workspace = WebSocketCommand::CreateWorkspace {
        data: CreateWorkspaceCommand {
            name: "我的工作空间".to_string(),
            url_key: "my-workspace".to_string(),
            logo_url: None,
        },
        request_id: Some("req-006".to_string()),
    };
    println!("      ✓ 创建工作空间命令已创建");

    // 序列化命令为 JSON
    println!("\n   📄 序列化示例:");
    let json = serde_json::to_string_pretty(&create_label)?;
    println!(
        "      {}",
        json.lines().take(10).collect::<Vec<_>>().join("\n      ")
    );

    Ok(())
}

/// 演示错误处理
fn demonstrate_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let _error_handler = WebSocketErrorHandler::new();

    println!("   ⚠️  错误类型演示:");

    // 模拟不同类型的错误
    let error_codes = vec![
        (WebSocketErrorCode::AuthenticationFailed, "认证失败"),
        (WebSocketErrorCode::TokenExpired, "令牌过期"),
        (WebSocketErrorCode::RateLimitExceeded, "超出速率限制"),
        (WebSocketErrorCode::CommandInvalid, "无效命令"),
        (WebSocketErrorCode::CommandFailed, "命令失败"),
        (WebSocketErrorCode::PermissionDenied, "权限被拒"),
        (WebSocketErrorCode::InternalError, "内部错误"),
        (WebSocketErrorCode::ConnectionTimeout, "连接超时"),
        (WebSocketErrorCode::DuplicateRequest, "重复请求"),
    ];

    for (code, description) in error_codes {
        println!("      • {:?}: {}", code, description);
    }

    println!("\n   ✅ 错误处理器已就绪");

    Ok(())
}
