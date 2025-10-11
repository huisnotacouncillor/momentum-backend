use rust_backend::{
    config::Config,
    db::enums::LabelLevel,
    websocket::{
        WebSocketCommand,
        auth::AuthenticatedUser,
        commands::types::CreateLabelCommand,
        security::{MessageSigner, SecurityError},
    },
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 创建配置（在实际应用中从环境变量加载）
    let config = Config {
        database_url: "postgresql://user:password@localhost/momentum".to_string(),
        database_max_connections: 10,
        database_min_connections: 5,
        database_connection_timeout: 30,
        redis_url: "redis://localhost:6379".to_string(),
        redis_pool_size: 10,
        server_host: "localhost".to_string(),
        server_port: 8000,
        cors_origins: vec!["*".to_string()],
        jwt_secret: "your-super-secret-jwt-key-for-signing-messages".to_string(),
        jwt_access_token_expires_in: 3600,
        jwt_refresh_token_expires_in: 604800,
        log_level: "info".to_string(),
        log_format: "json".to_string(),
        assets_url: "http://localhost:8000/assets".to_string(),
        bcrypt_cost: 4,
    };

    println!("🚀 WebSocket安全功能演示");
    println!("================================");

    // 1. 创建消息签名器
    println!("\n1. 创建消息签名器...");
    let message_signer = MessageSigner::new(&config);
    println!("✅ 消息签名器创建成功");

    // 2. 创建测试用户
    let test_user = AuthenticatedUser {
        user_id: Uuid::new_v4(),
        username: "test_user".to_string(),
        email: "test@example.com".to_string(),
        name: "Test User".to_string(),
        avatar_url: None,
        current_workspace_id: Some(Uuid::new_v4()),
    };

    // 3. 创建测试命令
    println!("\n2. 创建测试命令...");
    let test_command = WebSocketCommand::CreateLabel {
        data: CreateLabelCommand {
            name: "重要标签".to_string(),
            color: "#FF0000".to_string(),
            level: LabelLevel::Project,
        },
        request_id: Some("test-command-123".to_string()),
    };

    let command_payload = serde_json::to_value(&test_command)?;
    println!("✅ 测试命令创建成功: {:?}", test_command);

    // 4. 签名消息
    println!("\n3. 对消息进行签名...");
    let secure_message = message_signer.sign_message(&command_payload, test_user.user_id);
    println!("✅ 消息签名成功");
    println!("   - 消息ID: {}", secure_message.message_id);
    println!("   - 时间戳: {}", secure_message.timestamp);
    println!("   - 随机数: {}", secure_message.nonce);
    println!("   - 签名: {}", secure_message.signature);

    // 5. 验证消息
    println!("\n4. 验证消息...");
    match message_signer.verify_message(&secure_message).await {
        Ok(()) => println!("✅ 消息验证成功"),
        Err(e) => println!("❌ 消息验证失败: {}", e),
    }

    // 6. 测试重放攻击检测
    println!("\n5. 测试重放攻击检测...");
    match message_signer.verify_message(&secure_message).await {
        Ok(()) => println!("❌ 重放攻击检测失败 - 消息被重复处理"),
        Err(SecurityError::ReplayAttack { message_id }) => {
            println!("✅ 重放攻击检测成功 - 阻止了重复消息: {}", message_id);
        }
        Err(e) => println!("❌ 其他错误: {}", e),
    }

    // 7. 测试消息篡改检测
    println!("\n6. 测试消息篡改检测...");
    let mut tampered_message = secure_message.clone();
    tampered_message.signature = "tampered_signature".to_string();

    match message_signer.verify_message(&tampered_message).await {
        Ok(()) => println!("❌ 篡改检测失败 - 篡改的消息通过了验证"),
        Err(SecurityError::InvalidSignature { .. }) => {
            println!("✅ 篡改检测成功 - 检测到签名被篡改");
        }
        Err(e) => println!("❌ 其他错误: {}", e),
    }

    // 8. 测试过期消息检测
    println!("\n7. 测试过期消息检测...");
    let mut expired_message = secure_message.clone();
    expired_message.timestamp = chrono::Utc::now().timestamp() - 1000; // 1000秒前

    match message_signer.verify_message(&expired_message).await {
        Ok(()) => println!("❌ 过期检测失败 - 过期消息通过了验证"),
        Err(SecurityError::MessageExpired { .. }) => {
            println!("✅ 过期检测成功 - 检测到消息已过期");
        }
        Err(e) => println!("❌ 其他错误: {}", e),
    }

    // 9. 演示命令处理器的安全功能
    println!("\n8. 演示命令处理器的安全功能...");

    // 注意：这里需要实际的数据库连接，所以我们只是演示概念
    println!("📝 在实际应用中，WebSocketCommandHandler会:");
    println!("   - 自动验证所有接收到的安全消息");
    println!("   - 拒绝未签名的消息");
    println!("   - 防止重放攻击");
    println!("   - 确保消息的完整性和真实性");

    println!("\n🎉 安全功能演示完成！");
    println!("\n📋 安全功能总结:");
    println!("   ✅ HMAC-SHA256消息签名");
    println!("   ✅ 防重放攻击保护");
    println!("   ✅ 时间戳验证");
    println!("   ✅ 消息完整性检查");
    println!("   ✅ 自动缓存清理");

    Ok(())
}

/// 演示如何在WebSocket连接中使用安全功能
#[allow(dead_code)]
async fn demo_websocket_security_usage() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔧 WebSocket安全功能使用示例:");

    println!("\n客户端发送消息的步骤:");
    println!("1. 创建命令对象");
    println!("2. 序列化为JSON");
    println!("3. 使用MessageSigner签名消息");
    println!("4. 发送SecureMessage到服务器");

    println!("\n服务器处理消息的步骤:");
    println!("1. 接收SecureMessage");
    println!("2. 使用MessageSigner验证签名");
    println!("3. 检查时间戳和重放攻击");
    println!("4. 解析命令并执行");
    println!("5. 返回响应");

    println!("\n安全配置建议:");
    println!("- 使用强JWT密钥 (至少32字符)");
    println!("- 设置合理的时间窗口 (5-10分钟)");
    println!("- 定期清理消息ID缓存");
    println!("- 监控安全事件和异常");

    Ok(())
}
