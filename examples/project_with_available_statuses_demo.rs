//! 演示如何获取项目的可用状态
//!
//! 这个示例展示了如何在创建或更新问题时获取项目的可用状态列表

use uuid::Uuid;

fn main() {
    println!("🚀 项目状态演示");
    println!("================================");

    // 创建示例项目
    let project_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();

    println!("\n示例项目:");
    println!("  项目ID: {}", project_id);
    println!("  工作空间ID: {}", workspace_id);

    // 在实际应用中，你会从数据库查询项目的可用状态
    // 示例查询模式:
    println!("\n📝 查询项目可用状态的步骤:");
    println!("1. 根据项目ID查询项目信息");
    println!("2. 根据项目的 project_status_id 查询项目状态");
    println!("3. 使用 ProjectStatusesRepo::list_by_workspace 获取工作空间的所有状态");
    println!("4. 根据状态类别筛选可用状态");

    // 状态类别说明
    println!("\n📊 项目状态类别:");
    println!("  - Backlog: 待办");
    println!("  - Planned: 计划中");
    println!("  - InProgress: 进行中");
    println!("  - Completed: 已完成");
    println!("  - Canceled: 已取消");

    println!("\n✅ 演示完成!");
    println!("\n💡 提示: 在实际应用中，需要:");
    println!("  1. 建立数据库连接");
    println!("  2. 使用 ProjectRepo 和 ProjectStatusesRepo 进行查询");
    println!("  3. 根据项目的当前状态返回合适的状态转换选项");
}
