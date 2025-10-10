use serde_json::json;

/// 演示 issues WebSocket 命令的使用
fn main() {
    println!("Issues WebSocket Commands Demo");
    println!("================================");

    // 创建 issue 命令示例
    let create_issue_cmd = json!({
        "type": "create_issue",
        "data": {
            "title": "修复登录页面bug",
            "description": "用户反馈登录页面在某些浏览器上无法正常显示",
            "team_id": "550e8400-e29b-41d4-a716-446655440000",
            "project_id": "550e8400-e29b-41d4-a716-446655440001",
            "priority": "high",
            "assignee_id": "550e8400-e29b-41d4-a716-446655440002",
            "label_ids": [
                "550e8400-e29b-41d4-a716-446655440003",
                "550e8400-e29b-41d4-a716-446655440004"
            ]
        },
        "request_id": "req_001"
    });

    println!("1. 创建 Issue 命令:");
    println!("{}", serde_json::to_string_pretty(&create_issue_cmd).unwrap());

    // 更新 issue 命令示例
    let update_issue_cmd = json!({
        "type": "update_issue",
        "issue_id": "550e8400-e29b-41d4-a716-446655440005",
        "data": {
            "title": "修复登录页面bug - 更新",
            "description": "已定位到问题，正在修复中",
            "priority": "medium",
            "assignee_id": "550e8400-e29b-41d4-a716-446655440002"
        },
        "request_id": "req_002"
    });

    println!("\n2. 更新 Issue 命令:");
    println!("{}", serde_json::to_string_pretty(&update_issue_cmd).unwrap());

    // 查询 issues 命令示例
    let query_issues_cmd = json!({
        "type": "query_issues",
        "filters": {
            "team_id": "550e8400-e29b-41d4-a716-446655440000",
            "project_id": "550e8400-e29b-41d4-a716-446655440001",
            "assignee_id": "550e8400-e29b-41d4-a716-446655440002",
            "priority": "high",
            "search": "登录"
        },
        "request_id": "req_003"
    });

    println!("\n3. 查询 Issues 命令:");
    println!("{}", serde_json::to_string_pretty(&query_issues_cmd).unwrap());

    // 获取单个 issue 命令示例
    let get_issue_cmd = json!({
        "type": "get_issue",
        "issue_id": "550e8400-e29b-41d4-a716-446655440005",
        "request_id": "req_004"
    });

    println!("\n4. 获取 Issue 命令:");
    println!("{}", serde_json::to_string_pretty(&get_issue_cmd).unwrap());

    // 删除 issue 命令示例
    let delete_issue_cmd = json!({
        "type": "delete_issue",
        "issue_id": "550e8400-e29b-41d4-a716-446655440005",
        "request_id": "req_005"
    });

    println!("\n5. 删除 Issue 命令:");
    println!("{}", serde_json::to_string_pretty(&delete_issue_cmd).unwrap());

    println!("\n支持的功能:");
    println!("- 创建 Issue (create_issue)");
    println!("- 更新 Issue (update_issue)");
    println!("- 删除 Issue (delete_issue)");
    println!("- 查询 Issues (query_issues)");
    println!("- 获取单个 Issue (get_issue)");

    println!("\n支持的过滤条件:");
    println!("- team_id: 按团队过滤");
    println!("- project_id: 按项目过滤");
    println!("- assignee_id: 按指派人过滤");
    println!("- priority: 按优先级过滤 (none, low, medium, high, urgent)");
    println!("- search: 按标题搜索");
}
