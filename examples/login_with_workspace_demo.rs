/// 登录/注册返回工作空间 URL Key 功能演示
///
/// 此示例演示：
/// 1. 用户登录并获取工作空间 URL key
/// 2. 根据 URL key 判断用户状态
/// 3. 展示前端如何使用这个信息
///
/// 运行方式：
/// ```bash
/// cargo run --example login_with_workspace_demo
/// ```
use reqwest;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 登录返回工作空间 URL Key 演示 ===\n");

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
        println!("\n提示: 请确保服务器正在运行，并且已创建测试用户");
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

    println!("✅ 登录成功!\n");

    // 步骤 2: 检查响应数据
    println!("2. 检查响应数据...");

    let user = &login_result["data"]["user"];
    let current_workspace_url_key = login_result["data"]["current_workspace_url_key"].as_str();

    println!("   用户信息:");
    println!("   - ID: {}", user["id"]);
    println!("   - 用户名: {}", user["username"]);
    println!("   - 邮箱: {}", user["email"]);
    println!("   - 姓名: {}", user["name"]);
    println!();

    // 步骤 3: 根据工作空间状态做出不同处理
    println!("3. 工作空间状态检查...");

    match current_workspace_url_key {
        Some(url_key) => {
            println!("   ✅ 用户有当前工作空间");
            println!("   - URL Key: {}", url_key);
            println!();
            println!("   📱 前端应该这样处理:");
            println!("   ```javascript");
            println!("   // 保存 tokens");
            println!("   localStorage.setItem('access_token', data.access_token);");
            println!("   localStorage.setItem('refresh_token', data.refresh_token);");
            println!();
            println!("   // 跳转到工作空间");
            println!(
                "   window.location.href = `/workspace/{}/dashboard`;",
                url_key
            );
            println!("   ```");
            println!();
            println!("   🔗 可能的跳转路径:");
            println!("   - /workspace/{}/dashboard", url_key);
            println!("   - /workspace/{}/issues", url_key);
            println!("   - /workspace/{}/projects", url_key);
        }
        None => {
            println!("   ℹ️  用户暂无工作空间");
            println!();
            println!("   📱 前端应该这样处理:");
            println!("   ```javascript");
            println!("   // 保存 tokens");
            println!("   localStorage.setItem('access_token', data.access_token);");
            println!("   localStorage.setItem('refresh_token', data.refresh_token);");
            println!();
            println!("   // 引导用户创建或加入工作空间");
            println!("   window.location.href = '/onboarding/workspace';");
            println!("   ```");
            println!();
            println!("   🔗 可能的跳转路径:");
            println!("   - /onboarding/workspace - 引导页");
            println!("   - /workspaces/create - 创建工作空间");
            println!("   - /workspaces/join - 加入工作空间");
        }
    }
    println!();

    // 步骤 4: 展示完整的前端集成示例
    println!("4. 前端集成示例...\n");

    println!("   TypeScript 类型定义:");
    println!("   ```typescript");
    println!("   interface LoginResponse {{");
    println!("     access_token: string;");
    println!("     refresh_token: string;");
    println!("     token_type: string;");
    println!("     expires_in: number;");
    println!("     user: {{");
    println!("       id: string;");
    println!("       email: string;");
    println!("       username: string;");
    println!("       name: string;");
    println!("       avatar_url: string | null;");
    println!("     }};");
    println!("     current_workspace_url_key: string | null;");
    println!("   }}");
    println!("   ```");
    println!();

    println!("   React 路由处理示例:");
    println!("   ```typescript");
    println!("   const handleLogin = async (email: string, password: string) => {{");
    println!("     const response = await authService.login(email, password);");
    println!("     ");
    println!("     // 保存认证信息");
    println!("     localStorage.setItem('access_token', response.access_token);");
    println!("     localStorage.setItem('refresh_token', response.refresh_token);");
    println!("     ");
    println!("     // 根据工作空间状态跳转");
    println!("     if (response.current_workspace_url_key) {{");
    println!("       navigate(`/workspace/${{response.current_workspace_url_key}}/dashboard`);");
    println!("     }} else {{");
    println!("       navigate('/onboarding');");
    println!("     }}");
    println!("   }};");
    println!("   ```");
    println!();

    // 总结
    println!("=== 功能总结 ===\n");
    println!("✨ 优势:");
    println!("  1. 前端无需额外请求即可知道用户的工作空间");
    println!("  2. 登录后可以直接跳转到正确的页面");
    println!("  3. 简化了新用户的引导流程");
    println!("  4. 减少了 API 调用次数");
    println!();
    println!("📝 使用场景:");
    println!("  - 登录后自动跳转到工作空间主页");
    println!("  - 注册后引导用户创建/加入工作空间");
    println!("  - 多工作空间切换后的页面更新");
    println!();
    println!("🔒 注意事项:");
    println!("  - url_key 可能为 null（新用户或未加入工作空间）");
    println!("  - 需要在前端做好 null 值处理");
    println!("  - 建议缓存 url_key 到本地存储");

    Ok(())
}
