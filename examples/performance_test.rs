use rust_backend::config::Config;
use rust_backend::db::models::auth::User;
use rust_backend::utils::AssetUrlHelper;
use std::time::Instant;
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config = Config::from_env()?;

    // 创建资源 URL 处理工具
    let asset_helper = AssetUrlHelper::new(&config.assets());

    // 创建测试用户
    let user = User {
        id: Uuid::new_v4(),
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
        username: "testuser".to_string(),
        avatar_url: Some("avatars/test-user.jpg".to_string()),
        is_active: true,
        created_at: chrono::DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .naive_utc(),
        updated_at: chrono::DateTime::from_timestamp(1640995200, 0)
            .unwrap()
            .naive_utc(),
        current_workspace_id: None,
    };

    println!("=== 性能测试：Avatar URL 处理 ===");
    println!("Assets base URL: {}", config.assets().base_url);
    println!();

    // 测试 1: 重复创建 AssetUrlHelper 的性能影响
    println!("测试 1: 重复创建 AssetUrlHelper 的性能影响");
    let iterations = 10000;

    // 模拟旧的实现（每次创建新的 AssetUrlHelper）
    let start = Instant::now();
    for _ in 0..iterations {
        let _helper = AssetUrlHelper::new(&config.assets());
        let _ = user.get_processed_avatar_url(&_helper);
    }
    let old_duration = start.elapsed();

    // 模拟新的实现（重用 AssetUrlHelper）
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = user.get_processed_avatar_url(&asset_helper);
    }
    let new_duration = start.elapsed();

    println!("旧实现 ({} 次): {:?}", iterations, old_duration);
    println!("新实现 ({} 次): {:?}", iterations, new_duration);
    println!(
        "性能提升: {:.2}x",
        old_duration.as_nanos() as f64 / new_duration.as_nanos() as f64
    );
    println!();

    // 测试 2: 字符串操作优化
    println!("测试 2: 字符串操作优化");
    let url_paths = vec![
        "avatars/user1.jpg",
        "avatars/user2.png",
        "team-icons/team1.jpg",
        "project-icons/project1.png",
        "attachments/doc1.pdf",
    ];

    let start = Instant::now();
    for _ in 0..iterations {
        for path in &url_paths {
            let _ = asset_helper.build_url(path);
        }
    }
    let build_duration = start.elapsed();

    println!(
        "构建 {} 个 URL ({} 次): {:?}",
        url_paths.len(),
        iterations,
        build_duration
    );
    println!();

    // 测试 3: 外部链接 vs 内部路径处理
    println!("测试 3: 外部链接 vs 内部路径处理");
    let external_url = "https://gravatar.com/avatar/user123.jpg";
    let internal_path = "avatars/user123.jpg";

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = asset_helper.process_url(external_url);
    }
    let external_duration = start.elapsed();

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = asset_helper.process_url(internal_path);
    }
    let internal_duration = start.elapsed();

    println!("外部链接处理 ({} 次): {:?}", iterations, external_duration);
    println!("内部路径处理 ({} 次): {:?}", iterations, internal_duration);
    println!(
        "外部链接处理速度比: {:.2}x",
        internal_duration.as_nanos() as f64 / external_duration.as_nanos() as f64
    );
    println!();

    // 测试 4: 内存分配优化
    println!("测试 4: 内存分配优化");
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = user.get_processed_avatar_url(&asset_helper);
    }
    let string_duration = start.elapsed();

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = user.get_processed_avatar_url_ref(&asset_helper);
    }
    let cow_duration = start.elapsed();

    println!("String 返回 ({} 次): {:?}", iterations, string_duration);
    println!("Cow 返回 ({} 次): {:?}", iterations, cow_duration);
    println!(
        "Cow 优化提升: {:.2}x",
        string_duration.as_nanos() as f64 / cow_duration.as_nanos() as f64
    );
    println!();

    // 总结
    println!("=== 优化总结 ===");
    println!("1. 避免重复创建 AssetUrlHelper 实例");
    println!("2. 预计算 base_url_with_slash 避免重复字符串操作");
    println!("3. 外部链接直接返回，避免不必要的字符串构建");
    println!("4. 使用 Cow<str> 避免不必要的字符串分配");
    println!("5. 将 AssetUrlHelper 存储在 AppState 中全局复用");

    Ok(())
}
