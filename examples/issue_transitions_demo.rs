use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Issue Workflow Transitions Demo ===\n");

    // Get the base URL from environment variable or use default
    let base_url = env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    println!("1. API接口说明:");
    println!("   GET /issues/{{issue_id}}/transitions - 获取指定issue可用的workflow transitions");
    println!();

    println!("2. 请求示例:");
    println!("   curl -H \"Authorization: Bearer <token>\" \\");
    println!("        \"{}/issues/{{issue_id}}/transitions\"", base_url);
    println!();

    println!("3. 响应格式:");
    println!("   {{");
    println!("     \"success\": true,");
    println!("     \"message\": \"Available transitions retrieved successfully\",");
    println!("     \"data\": [");
    println!("       {{");
    println!("         \"id\": \"transition-uuid\",");
    println!("         \"workflow_id\": \"workflow-uuid\",");
    println!("         \"from_state_id\": \"from-state-uuid\", // 可为null");
    println!("         \"to_state_id\": \"to-state-uuid\",");
    println!("         \"name\": \"Move to Done\",");
    println!("         \"description\": \"Move issue to completed state\",");
    println!("         \"created_at\": \"2025-01-01T00:00:00Z\",");
    println!("         \"from_state\": {{");
    println!("           \"id\": \"from-state-uuid\",");
    println!("           \"name\": \"In Progress\",");
    println!("           \"category\": \"started\",");
    println!("           \"color\": \"#F1BF00\",");
    println!("           \"position\": 1");
    println!("         }}, // 可为null");
    println!("         \"to_state\": {{");
    println!("           \"id\": \"to-state-uuid\",");
    println!("           \"name\": \"Done\",");
    println!("           \"category\": \"completed\",");
    println!("           \"color\": \"#0000FF\",");
    println!("           \"position\": 1");
    println!("         }}");
    println!("       }}");
    println!("     ]");
    println!("   }}");
    println!();

    println!("4. 功能说明:");
    println!("   - 根据issue的当前workflow state，返回所有可用的状态转换");
    println!("   - 包含转换的详细信息：名称、描述、源状态、目标状态");
    println!("   - 支持从任何状态转换（from_state_id为null）");
    println!("   - 支持从特定状态转换（from_state_id匹配当前状态）");
    println!("   - 自动验证用户对issue的访问权限");
    println!();

    println!("5. 使用场景:");
    println!("   - 前端显示issue状态转换按钮");
    println!("   - 工作流引擎确定可执行的操作");
    println!("   - 状态机验证状态转换的有效性");
    println!("   - 用户界面动态生成状态选择器");
    println!();

    println!("6. 错误处理:");
    println!("   - 400: 未选择工作空间或issue没有关联workflow");
    println!("   - 404: issue不存在或无访问权限");
    println!("   - 500: 数据库连接失败或查询错误");
    println!();

    println!("7. 注意事项:");
    println!("   - 需要有效的认证token");
    println!("   - issue必须属于用户当前工作空间");
    println!("   - issue必须有关联的workflow");
    println!("   - 返回的transitions包含完整的状态信息");
    println!();

    Ok(())
}
