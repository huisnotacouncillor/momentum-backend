//! Workflow Demo
//!
//! 这个示例演示了如何使用工作流功能
//!
//! 运行方式: cargo run --example workflow_demo

fn main() {
    println!("=== Workflow Demo ===\n");

    println!("1. 工作流状态分类:");
    println!("   - Backlog: 待办事项");
    println!("   - Unstarted: 未开始");
    println!("   - Started: 已开始");
    println!("   - Completed: 已完成");
    println!("   - Canceled: 已取消");
    println!("   - Triage: 分类处理");

    println!("\n2. 创建请求结构体示例:");

    println!("   CreateWorkflowRequest:");
    println!("   - name: 开发团队工作流");
    println!("   - description: 用于开发团队的标准工作流程");
    println!("   - is_default: true");

    println!("\n   CreateWorkflowStateRequest:");
    println!("   - name: 代码审查");
    println!("   - description: 等待代码审查的状态");
    println!("   - color: #FFAA00");
    println!("   - category: Started");
    println!("   - position: 2");
    println!("   - is_default: false");

    println!("\n3. 响应结构体示例:");

    println!("   WorkflowResponse:");
    println!("   - id: UUID");
    println!("   - name: 示例工作流");
    println!("   - description: 这是一个示例工作流");
    println!("   - team_id: UUID");
    println!("   - is_default: true");
    println!("   - states: []");

    println!("\n   WorkflowStateResponse:");
    println!("   - id: UUID");
    println!("   - workflow_id: UUID");
    println!("   - name: 待办");
    println!("   - description: 等待处理的任务");
    println!("   - color: #6666FF");
    println!("   - category: Unstarted");
    println!("   - position: 1");
    println!("   - is_default: true");

    println!("\n4. API 端点示例:");
    println!("   GET    /teams/{{team_id}}/workflows              - 获取团队的所有工作流");
    println!("   POST   /teams/{{team_id}}/workflows              - 创建新工作流");
    println!("   GET    /workflows/{{workflow_id}}                - 获取特定工作流详情");
    println!("   PUT    /workflows/{{workflow_id}}                - 更新工作流");
    println!("   DELETE /workflows/{{workflow_id}}                - 删除工作流");
    println!("   GET    /workflows/{{workflow_id}}/states         - 获取工作流的所有状态");
    println!("   POST   /workflows/{{workflow_id}}/states         - 创建工作流状态");

    println!("\n5. 数据库表结构:");
    println!("   - workflows: 工作流主表");
    println!("     * id: UUID (主键)");
    println!("     * name: VARCHAR(255)");
    println!("     * description: TEXT");
    println!("     * team_id: UUID (外键)");
    println!("     * is_default: BOOLEAN");
    println!("     * created_at: TIMESTAMPTZ");
    println!("     * updated_at: TIMESTAMPTZ");

    println!("\n   - workflow_states: 工作流状态表");
    println!("     * id: UUID (主键)");
    println!("     * workflow_id: UUID (外键)");
    println!("     * name: VARCHAR(255)");
    println!("     * description: TEXT");
    println!("     * color: VARCHAR(7)");
    println!("     * category: VARCHAR(50)");
    println!("     * position: INTEGER");
    println!("     * is_default: BOOLEAN");
    println!("     * created_at: TIMESTAMPTZ");
    println!("     * updated_at: TIMESTAMPTZ");

    println!("\n   - workflow_transitions: 状态转换表");
    println!("     * id: UUID (主键)");
    println!("     * workflow_id: UUID (外键)");
    println!("     * from_state_id: UUID (外键，可为空)");
    println!("     * to_state_id: UUID (外键)");
    println!("     * name: VARCHAR(255)");
    println!("     * description: TEXT");
    println!("     * created_at: TIMESTAMPTZ");

    println!("\n   - issues: 问题表（新增字段）");
    println!("     * workflow_id: UUID (外键，可为空)");
    println!("     * workflow_state_id: UUID (外键，可为空)");

    println!("\n6. 使用场景:");
    println!("   - 团队可以自定义工作流程");
    println!("   - 支持状态转换规则");
    println!("   - 问题可以关联到特定的工作流和状态");
    println!("   - 支持默认工作流设置");
    println!("   - 状态可以按分类组织");

    println!("\n=== 演示完成 ===");
}
