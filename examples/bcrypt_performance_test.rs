use bcrypt::{hash, verify};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Bcrypt 性能测试 ===");
    println!();

    let password = "123456";
    let iterations = 5;

    // 测试不同的bcrypt cost
    let costs = vec![4, 6, 8, 10, 12];

    for cost in costs {
        println!("测试 bcrypt cost = {}", cost);

        let mut hash_times = Vec::new();
        let mut verify_times = Vec::new();

        // 测试哈希性能
        for i in 1..=iterations {
            let start = Instant::now();
            let hashed = hash(password.as_bytes(), cost)?;
            let hash_time = start.elapsed();
            hash_times.push(hash_time);

            println!("  哈希 {}: {:?}", i, hash_time);

            // 测试验证性能
            let start = Instant::now();
            let is_valid = verify(password.as_bytes(), &hashed)?;
            let verify_time = start.elapsed();
            verify_times.push(verify_time);

            println!("  验证 {}: {:?} (有效: {})", i, verify_time, is_valid);
        }

        let avg_hash = hash_times.iter().sum::<std::time::Duration>() / iterations;
        let avg_verify = verify_times.iter().sum::<std::time::Duration>() / iterations;

        println!("  平均哈希时间: {:?}", avg_hash);
        println!("  平均验证时间: {:?}", avg_verify);
        println!("  总平均时间: {:?}", avg_hash + avg_verify);
        println!();
    }

    println!("=== 性能分析 ===");
    println!("Cost 4-6: 开发环境推荐 (快速)");
    println!("Cost 8-10: 测试环境推荐 (平衡)");
    println!("Cost 12+: 生产环境推荐 (安全)");
    println!();
    println!("建议:");
    println!("- 开发环境使用 cost=6 或 cost=8");
    println!("- 生产环境使用 cost=12 或更高");
    println!("- 可以通过环境变量 BCRYPT_COST 配置");

    Ok(())
}
