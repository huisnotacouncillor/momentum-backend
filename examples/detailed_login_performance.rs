use std::time::Instant;
use reqwest::Client;
use serde_json::json;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 详细登录性能分析 ===");
    println!();

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let base_url = "http://localhost:8000";
    let login_data = json!({
        "email": "huisnota54@gmail.com",
        "password": "123456"
    });

    // 测试单个请求的详细时间分析
    println!("执行单次登录请求详细分析...");

    let start = Instant::now();

    // 测量DNS解析和连接建立时间
    let dns_start = Instant::now();
    let response = client
        .post(&format!("{}/auth/login", base_url))
        .json(&login_data)
        .send()
        .await?;
    let total_time = start.elapsed();

    println!("总请求时间: {:?}", total_time);
    println!("HTTP状态: {}", response.status());
    println!();

    // 分析响应时间分布
    let response_text = response.text().await?;
    println!("响应大小: {} bytes", response_text.len());

    // 解析响应以检查是否有性能信息
    if let Ok(response_json) = serde_json::from_str::<serde_json::Value>(&response_text) {
        println!("响应结构: {}", serde_json::to_string_pretty(&response_json)?);
    }

    println!();

    // 执行多次请求来识别模式
    println!("执行 5 次连续请求分析...");
    let mut times = Vec::new();

    for i in 1..=5 {
        let start = Instant::now();
        let response = client
            .post(&format!("{}/auth/login", base_url))
            .json(&login_data)
            .send()
            .await?;
        let duration = start.elapsed();
        times.push(duration);

        println!("请求 {}: {:?} (状态: {})", i, duration, response.status());

        // 短暂延迟避免服务器过载
        sleep(std::time::Duration::from_millis(100)).await;
    }

    let avg_time = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    let min_time = *times.iter().min().unwrap();
    let max_time = *times.iter().max().unwrap();

    println!();
    println!("=== 性能分析结果 ===");
    println!("平均时间: {:?}", avg_time);
    println!("最小时间: {:?}", min_time);
    println!("最大时间: {:?}", max_time);
    println!("时间变化: {:?}", max_time - min_time);

    // 性能瓶颈分析
    println!();
    println!("=== 性能瓶颈分析 ===");

    if avg_time.as_millis() > 500 {
        println!("❌ 主要瓶颈: 平均响应时间 > 500ms");

        if avg_time.as_millis() > 700 {
            println!("   - 可能原因: bcrypt密码验证 (通常需要 100-500ms)");
            println!("   - 可能原因: 数据库查询慢 (索引问题或连接慢)");
            println!("   - 可能原因: 网络延迟");
        }

        println!("建议优化:");
        println!("1. 检查服务器日志中的详细性能信息");
        println!("2. 验证数据库索引是否生效");
        println!("3. 考虑降低bcrypt cost (如果安全允许)");
        println!("4. 检查数据库连接池配置");
        println!("5. 考虑使用异步密码验证");
    } else {
        println!("✅ 性能良好: 平均响应时间 < 500ms");
    }

    // 稳定性分析
    let time_variance = max_time - min_time;
    if time_variance.as_millis() > 100 {
        println!("⚠️  稳定性问题: 响应时间变化 > 100ms");
        println!("   - 可能原因: 数据库连接池竞争");
        println!("   - 可能原因: 服务器资源竞争");
    } else {
        println!("✅ 稳定性良好: 响应时间变化 < 100ms");
    }

    Ok(())
}
