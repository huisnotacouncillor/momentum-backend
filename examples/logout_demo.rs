/// 登出功能演示
///
/// 此示例演示如何使用登出 API，包括：
/// 1. 用户登录获取 token
/// 2. 使用 token 访问受保护资源
/// 3. 用户登出并清除缓存
/// 4. 验证登出后无法访问受保护资源
///
/// 运行方式：
/// ```bash
/// cargo run --example logout_demo
/// ```
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    access_token: String,
    refresh_token: String,
    token_type: String,
    expires_in: i64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 登出功能演示 ===\n");

    let base_url = "http://localhost:8000";
    let client = reqwest::Client::new();

    // 步骤 1: 用户登录
    println!("1. 用户登录...");
    let login_payload = LoginRequest {
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
    };

    let login_response = client
        .post(&format!("{}/auth/login", base_url))
        .json(&login_payload)
        .send()
        .await?;

    if !login_response.status().is_success() {
        println!("❌ 登录失败: {}", login_response.status());
        println!("提示: 请确保服务器正在运行，并且已创建测试用户");
        println!("可以使用以下命令创建测试用户:");
        println!("  curl -X POST http://localhost:8000/auth/register \\");
        println!("    -H 'Content-Type: application/json' \\");
        println!("    -d '{{");
        println!("      \"email\": \"test@example.com\",");
        println!("      \"username\": \"testuser\",");
        println!("      \"name\": \"Test User\",");
        println!("      \"password\": \"password123\"");
        println!("    }}'");
        return Ok(());
    }

    let login_result: Value = login_response.json().await?;
    let access_token = login_result["data"]["access_token"]
        .as_str()
        .ok_or("Failed to get access token")?
        .to_string();

    println!("✅ 登录成功!");
    println!("   Access Token: {}...", &access_token[..50]);
    println!();

    // 步骤 2: 访问受保护资源（获取用户资料）
    println!("2. 使用 token 访问受保护资源...");
    let profile_response = client
        .get(&format!("{}/auth/profile", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    if profile_response.status().is_success() {
        let profile: Value = profile_response.json().await?;
        println!("✅ 成功获取用户资料:");
        println!("   用户名: {}", profile["data"]["username"]);
        println!("   邮箱: {}", profile["data"]["email"]);
        println!("   姓名: {}", profile["data"]["name"]);
    } else {
        println!("❌ 获取资料失败: {}", profile_response.status());
    }
    println!();

    // 步骤 3: 用户登出
    println!("3. 执行登出操作...");
    let logout_response = client
        .post(&format!("{}/auth/logout", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    if logout_response.status().is_success() {
        let logout_result: Value = logout_response.json().await?;
        println!("✅ 登出成功!");
        println!("   消息: {}", logout_result["message"]);
        println!();
        println!("   已完成的操作:");
        println!("   - 使数据库中的所有会话失效");
        println!("   - 清除 Redis 中的用户缓存");
        println!("   - 清除 Redis 中的用户资料缓存");
        println!("   - 清除 Redis 中的工作空间缓存");
    } else {
        println!("❌ 登出失败: {}", logout_response.status());
    }
    println!();

    // 步骤 4: 验证登出后无法访问受保护资源
    println!("4. 验证登出后的状态...");
    let verify_response = client
        .get(&format!("{}/auth/profile", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    if verify_response.status().is_success() {
        println!("⚠️  警告: 登出后仍然可以访问资源（这不应该发生）");
    } else {
        println!("✅ 验证通过: 登出后无法访问受保护资源");
        println!("   状态码: {}", verify_response.status());
    }
    println!();

    // 总结
    println!("=== 登出流程完成 ===");
    println!();
    println!("📝 登出最佳实践:");
    println!("  1. 客户端收到登出成功响应后应立即删除本地 token");
    println!("  2. 重定向用户到登录页面");
    println!("  3. 清除所有本地存储的用户相关数据");
    println!("  4. 如果使用 Redux/Vuex 等状态管理，清空用户状态");
    println!();
    println!("🔒 安全特性:");
    println!("  - 所有设备的会话都会失效（多设备登出）");
    println!("  - Redis 缓存会被立即清除");
    println!("  - 即使 Redis 失败，数据库会话仍然失效");
    println!("  - 支持优雅降级（缓存清理失败不影响登出）");

    Ok(())
}
