use reqwest::Client;
use serde_json::json;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 优化后的登录性能测试 ===");
    println!();

    let client = Client::new();
    let login_data = json!({
        "email": "huisnota54@gmail.com",
        "password": "123456"
    });

    // 执行多次测试
    let iterations = 5;
    let mut times = Vec::new();

    println!("执行 {} 次登录请求测试...", iterations);

    for i in 1..=iterations {
        let start = Instant::now();

        let response = client
            .post("http://localhost:8000/auth/login")
            .json(&login_data)
            .send()
            .await?;

        let duration = start.elapsed();
        times.push(duration);

        println!("请求 {}: {:?} (状态: {})", i, duration, response.status());

        // 短暂延迟
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let avg_time = times.iter().sum::<std::time::Duration>() / times.len() as u32;
    let min_time = *times.iter().min().unwrap();
    let max_time = *times.iter().max().unwrap();

    println!();
    println!("=== 性能测试结果 ===");
    println!("平均时间: {:?}", avg_time);
    println!("最小时间: {:?}", min_time);
    println!("最大时间: {:?}", max_time);
    println!("时间变化: {:?}", max_time - min_time);

    // 性能评估
    println!();
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

    // 与之前的对比
    println!();
    println!("=== 优化效果对比 ===");
    println!("优化前: ~750ms");
    println!("优化后: {:?}", avg_time);

    if avg_time.as_millis() < 750 {
        let improvement = ((750.0 - avg_time.as_millis() as f64) / 750.0 * 100.0) as u32;
        println!("性能提升: {}%", improvement);
    } else {
        println!("⚠️  性能没有明显改善，可能需要检查配置");
    }

    Ok(())
}
