use rust_backend::config::Config;
use rust_backend::utils::AssetUrlHelper;
use rust_backend::db::models::auth::{User, UserProfile};
use rust_backend::db::models::team::TeamInfo;
use rust_backend::db::models::workspace::WorkspaceInfo;
use uuid::Uuid;
use chrono::NaiveDateTime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config = Config::from_env()?;

    // 创建资源 URL 处理工具
    let asset_helper = AssetUrlHelper::new(&config.assets());

    println!("=== GET /profile 接口资源 URL 处理演示 ===");
    println!("Assets base URL: {}", config.assets().base_url);
    println!();

    // 模拟用户数据
    let user = User {
        id: Uuid::new_v4(),
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
        username: "zhangsan".to_string(),
        avatar_url: Some("avatars/zhangsan.jpg".to_string()),
        is_active: true,
        created_at: NaiveDateTime::from_timestamp_opt(1640995200, 0).unwrap(),
        updated_at: NaiveDateTime::from_timestamp_opt(1640995200, 0).unwrap(),
        current_workspace_id: Some(Uuid::new_v4()),
    };

    // 模拟团队数据
    let team_info = TeamInfo {
        id: Uuid::new_v4(),
        name: "开发团队".to_string(),
        team_key: "DEV".to_string(),
        description: Some("负责产品开发的团队".to_string()),
        icon_url: Some("team-icons/dev-team.png".to_string()),
        is_private: false,
        role: "member".to_string(),
    };

    // 模拟工作空间数据
    let workspace_info = WorkspaceInfo {
        id: Uuid::new_v4(),
        name: "示例工作空间".to_string(),
        url_key: "example-workspace".to_string(),
        logo_url: None,
    };

    // 模拟 UserProfile 响应（就像 GET /profile 接口返回的那样）
    let processed_avatar_url = user.get_processed_avatar_url(&asset_helper);
    let user_profile = UserProfile {
        id: user.id,
        email: user.email,
        username: user.username,
        name: user.name,
        avatar_url: processed_avatar_url,
        current_workspace_id: user.current_workspace_id,
        workspaces: vec![workspace_info],
        teams: vec![team_info],
    };

    println!("=== 用户资料信息 ===");
    println!("用户 ID: {}", user_profile.id);
    println!("用户名: {}", user_profile.username);
    println!("姓名: {}", user_profile.name);
    println!("邮箱: {}", user_profile.email);

    if let Some(avatar_url) = &user_profile.avatar_url {
        println!("头像 URL: {}", avatar_url);
    } else {
        println!("头像 URL: 未设置");
    }

    if let Some(workspace_id) = user_profile.current_workspace_id {
        println!("当前工作空间 ID: {}", workspace_id);
    } else {
        println!("当前工作空间 ID: 未设置");
    }

    println!();

    println!("=== 工作空间信息 ===");
    for workspace in &user_profile.workspaces {
        println!("工作空间: {} ({}), URL Key: {}",
                workspace.name, workspace.id, workspace.url_key);
    }

    println!();

    println!("=== 团队信息 ===");
    for team in &user_profile.teams {
        println!("团队: {} ({})", team.name, team.team_key);
        println!("  描述: {:?}", team.description);

        if let Some(icon_url) = &team.icon_url {
            println!("  图标 URL: {}", icon_url);
        } else {
            println!("  图标 URL: 未设置");
        }

        println!("  角色: {}", team.role);
        println!("  私有: {}", team.is_private);
        println!();
    }

    // 演示不同场景下的 URL 处理
    println!("=== URL 处理场景演示 ===");

    // 场景1: 内部头像路径
    let user_with_internal_avatar = User {
        id: Uuid::new_v4(),
        name: "李四".to_string(),
        email: "lisi@example.com".to_string(),
        username: "lisi".to_string(),
        avatar_url: Some("avatars/lisi.jpg".to_string()),
        is_active: true,
        created_at: NaiveDateTime::from_timestamp_opt(1640995200, 0).unwrap(),
        updated_at: NaiveDateTime::from_timestamp_opt(1640995200, 0).unwrap(),
        current_workspace_id: None,
    };

    println!("场景1 - 内部头像路径:");
    println!("  原始路径: {:?}", user_with_internal_avatar.avatar_url);
    if let Some(processed_url) = user_with_internal_avatar.get_processed_avatar_url(&asset_helper) {
        println!("  处理后 URL: {}", processed_url);
    }
    println!();

    // 场景2: 外部头像链接
    let user_with_external_avatar = User {
        id: Uuid::new_v4(),
        name: "王五".to_string(),
        email: "wangwu@example.com".to_string(),
        username: "wangwu".to_string(),
        avatar_url: Some("https://gravatar.com/avatar/wangwu.jpg".to_string()),
        is_active: true,
        created_at: NaiveDateTime::from_timestamp_opt(1640995200, 0).unwrap(),
        updated_at: NaiveDateTime::from_timestamp_opt(1640995200, 0).unwrap(),
        current_workspace_id: None,
    };

    println!("场景2 - 外部头像链接:");
    println!("  原始链接: {:?}", user_with_external_avatar.avatar_url);
    if let Some(processed_url) = user_with_external_avatar.get_processed_avatar_url(&asset_helper) {
        println!("  处理后 URL: {}", processed_url);
    }
    println!();

    // 场景3: 无头像
    let user_without_avatar = User {
        id: Uuid::new_v4(),
        name: "赵六".to_string(),
        email: "zhaoliu@example.com".to_string(),
        username: "zhaoliu".to_string(),
        avatar_url: None,
        is_active: true,
        created_at: NaiveDateTime::from_timestamp_opt(1640995200, 0).unwrap(),
        updated_at: NaiveDateTime::from_timestamp_opt(1640995200, 0).unwrap(),
        current_workspace_id: None,
    };

    println!("场景3 - 无头像:");
    println!("  原始值: {:?}", user_without_avatar.avatar_url);
    if let Some(processed_url) = user_without_avatar.get_processed_avatar_url(&asset_helper) {
        println!("  处理后 URL: {}", processed_url);
    } else {
        println!("  处理后 URL: None");
    }
    println!();

    println!("=== API 响应格式示例 ===");
    println!("GET /profile 接口返回的 JSON 格式:");
    println!("{{");
    println!("  \"success\": true,");
    println!("  \"message\": \"Profile retrieved successfully\",");
    println!("  \"data\": {{");
    println!("    \"id\": \"{}\",", user_profile.id);
    println!("    \"email\": \"{}\",", user_profile.email);
    println!("    \"username\": \"{}\",", user_profile.username);
    println!("    \"name\": \"{}\",", user_profile.name);
    println!("    \"avatar_url\": {:?},", user_profile.avatar_url);
    println!("    \"current_workspace_id\": {:?},", user_profile.current_workspace_id);
    println!("    \"workspaces\": [...],");
    println!("    \"teams\": [...]");
    println!("  }}");
    println!("}}");

    Ok(())
}