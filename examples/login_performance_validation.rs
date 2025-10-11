use reqwest::Client;
use serde_json::json;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 登录接口性能验证测试 ===");
    println!();

    let client = Client::new();
    let base_url = "http://localhost:8000";
    let login_data = json!({
        "email": "huisnota54@gmail.com",
        "password": "123456"
    });

    // 预热请求
    println!("执行预热请求...");
    let _ = client
        .post(format!("{}/auth/login", base_url))
        .json(&login_data)
        .send()
        .await;

    // 执行性能测试
    let iterations = 10;
    let mut total_time = std::time::Duration::new(0, 0);
    let mut min_time = std::time::Duration::from_secs(60);
    let mut max_time = std::time::Duration::new(0, 0);

    println!("执行 {} 次登录请求性能测试...", iterations);

    for i in 1..=iterations {
        let start = Instant::now();

        let response = client
            .post(format!("{}/auth/login", base_url))
            .json(&login_data)
            .send()
            .await?;

        let duration = start.elapsed();
        total_time += duration;

        if duration < min_time {
            min_time = duration;
        }
        if duration > max_time {
            max_time = duration;
        }

        println!("请求 {}: {:?} (状态: {})", i, duration, response.status());
    }

    let avg_time = total_time / iterations;

    println!();
    println!("=== 性能测试结果 ===");
    println!("总请求数: {}", iterations);
    println!("总时间: {:?}", total_time);
    println!("平均时间: {:?}", avg_time);
    println!("最小时间: {:?}", min_time);
    println!("最大时间: {:?}", max_time);
    println!();

    // 性能评估
    println!("=== 性能评估 ===");
    if avg_time.as_millis() < 100 {
        println!("✅ 优秀: 平均响应时间 < 100ms");
    } else if avg_time.as_millis() < 200 {
        println!("✅ 良好: 平均响应时间 < 200ms");
    } else if avg_time.as_millis() < 500 {
        println!("⚠️  一般: 平均响应时间 < 500ms");
    } else {
        println!("❌ 需要优化: 平均响应时间 >= 500ms");
    }

    if max_time.as_millis() < 200 {
        println!("✅ 稳定性优秀: 最大响应时间 < 200ms");
    } else if max_time.as_millis() < 500 {
        println!("⚠️  稳定性一般: 最大响应时间 < 500ms");
    } else {
        println!("❌ 稳定性差: 最大响应时间 >= 500ms");
    }

    println!();
    println!("=== 优化建议 ===");
    if avg_time.as_millis() > 200 {
        println!("1. 检查数据库连接池配置");
        println!("2. 验证数据库索引是否生效");
        println!("3. 检查网络延迟");
        println!("4. 考虑使用连接池预热");
    } else {
        println!("✅ 登录接口性能良好，无需进一步优化");
    }

    Ok(())
}
