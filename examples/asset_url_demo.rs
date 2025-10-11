use rust_backend::config::Config;
use rust_backend::db::models::auth::User;
use rust_backend::db::models::team::Team;
use rust_backend::utils::AssetUrlHelper;
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config = Config::from_env()?;

    // 创建资源 URL 处理工具
    let asset_helper = AssetUrlHelper::new(&config.assets());

    println!("Assets base URL: {}", config.assets().base_url);
    println!();

    // 演示通用 URL 构建方法
    println!("=== 通用 URL 构建方法演示 ===");

    // 构建头像 URL
    let avatar_url = asset_helper.build_avatar_url("user123.jpg");
    println!("头像 URL: {}", avatar_url);

    // 构建团队图标 URL
    let team_icon_url = asset_helper.build_team_icon_url("team456.png");
    println!("团队图标 URL: {}", team_icon_url);

    // 构建项目图标 URL
    let project_icon_url = asset_helper.build_project_icon_url("project789.png");
    println!("项目图标 URL: {}", project_icon_url);

    // 构建附件 URL
    let attachment_url = asset_helper.build_attachment_url("document.pdf");
    println!("附件 URL: {}", attachment_url);

    // 构建自定义路径 URL
    let custom_url = asset_helper.build_url("custom/path/image.jpg");
    println!("自定义路径 URL: {}", custom_url);

    println!();

    // 演示 URL 处理功能
    println!("=== URL 处理功能演示 ===");

    // 外部链接处理
    let external_url = "https://example.com/avatar.jpg";
    let processed_external = asset_helper.process_url(external_url);
    println!("外部链接: {} -> {}", external_url, processed_external);

    // 内部路径处理
    let internal_path = "avatars/user123.jpg";
    let processed_internal = asset_helper.process_url(internal_path);
    println!("内部路径: {} -> {}", internal_path, processed_internal);

    // 检查是否为外部链接
    println!("是否为外部链接:");
    println!(
        "  {}: {}",
        external_url,
        asset_helper.is_external_url(external_url)
    );
    println!(
        "  {}: {}",
        internal_path,
        asset_helper.is_external_url(internal_path)
    );

    println!();

    // 演示用户模型中的头像 URL 处理
    println!("=== 用户模型头像 URL 处理演示 ===");

    // 创建示例用户
    let user = User {
        id: Uuid::new_v4(),
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
        username: "zhangsan".to_string(),
        avatar_url: Some("avatars/zhangsan.jpg".to_string()),
        is_active: true,
        created_at: chrono::DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .naive_utc(),
        updated_at: chrono::DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .naive_utc(),
        current_workspace_id: None,
    };

    // 获取处理后的头像 URL
    if let Some(processed_avatar_url) = user.get_processed_avatar_url(&asset_helper) {
        println!("用户 {} 的头像 URL: {}", user.name, processed_avatar_url);
    } else {
        println!("用户 {} 没有设置头像", user.name);
    }

    // 演示外部头像 URL
    let user_with_external_avatar = User {
        id: Uuid::new_v4(),
        name: "李四".to_string(),
        email: "lisi@example.com".to_string(),
        username: "lisi".to_string(),
        avatar_url: Some("https://gravatar.com/avatar/lisi.jpg".to_string()),
        is_active: true,
        created_at: chrono::DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .naive_utc(),
        updated_at: chrono::DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .naive_utc(),
        current_workspace_id: None,
    };

    if let Some(processed_avatar_url) =
        user_with_external_avatar.get_processed_avatar_url(&asset_helper)
    {
        println!(
            "用户 {} 的头像 URL: {}",
            user_with_external_avatar.name, processed_avatar_url
        );
    }

    println!();

    // 演示团队模型中的图标 URL 处理
    println!("=== 团队模型图标 URL 处理演示 ===");

    let team = Team {
        id: Uuid::new_v4(),
        workspace_id: Uuid::new_v4(),
        name: "开发团队".to_string(),
        team_key: "DEV".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        description: Some("负责产品开发的团队".to_string()),
        icon_url: Some("team-icons/dev-team.png".to_string()),
        is_private: false,
    };

    if let Some(processed_icon_url) = team.get_processed_icon_url(&asset_helper) {
        println!("团队 {} 的图标 URL: {}", team.name, processed_icon_url);
    } else {
        println!("团队 {} 没有设置图标", team.name);
    }

    println!();

    // 演示环境变量配置
    println!("=== 环境变量配置说明 ===");
    println!("可以通过设置 ASSETS_URL 环境变量来配置资源基础 URL:");
    println!("  export ASSETS_URL=\"https://cdn.example.com/assets\"");
    println!("  export ASSETS_URL=\"http://localhost:3000/static\"");
    println!("如果不设置，将使用默认值: {}", config.assets().base_url);

    Ok(())
}
