# Momentum Backend 架构文档

> 完整的架构分析、审视与修复计划文档

---

## 📚 文档导航

| 文档 | 用途 | 何时阅读 |
|------|------|----------|
| **[ARCHITECTURE_ISSUES.md](./ARCHITECTURE_ISSUES.md)** | 7 个核心架构问题分析 | 想了解代码组织、技术债细节时 |
| **[ARCHITECTURE_REVIEW.md](./ARCHITECTURE_REVIEW.md)** | 架构师全面审视（12 维度） | 完整安全/运维/可观测性审视 |
| **[REFACTOR_PLAN.md](./REFACTOR_PLAN.md)** | 详细修复实施计划 | 准备开始修复工作 |

---

## 🎯 快速概览

### 问题严重程度分布

| 优先级 | 数量 | 关键问题 |
|--------|------|----------|
| 🔴 **P0 - 立即修复** | 6 | 工作区隔离失效、连接池 panic、缺少 RBAC |
| 🟠 **P1 - 高优** | 5 | Dockerfile healthcheck 失败、严重 N+1 查询 |
| 🟡 **P2 - 中优** | 8 | WebSocket 命令双重分发、服务层难测试 |
| 🟢 **P3 - 低优** | 4 | API 版本化、日志配置不生效 |

### 文档组织

```
ARCHITECTURE_ISSUES.md     ──►  问题详情（已勘误）
ARCHITECTURE_REVIEW.md     ──►  架构师审视（含安全/运维/可观测性）
REFACTOR_PLAN.md           ──►  修复计划（含代码示例）
```

---

## 🚨 最紧急的 3 个问题

### 1. 工作区隔离完全失效
`IssueRepo::find_by_id_in_workspace` 的 `_workspace_id` 参数（**下划线前缀**）被忽略，任何用户可跨工作区访问任意 Issue。

### 2. switch_workspace 跳过成员验证
代码自带 TODO 注释："For now, just update the current workspace"，允许切换到任意工作区后删除它。

### 3. Dockerfile 健康检查永远不会成功
HEALTHCHECK 调用 `/health`，但路由不存在 + curl 未安装，导致容器无限重启。

---

## 📖 阅读顺序建议

### 如果你是新加入的开发者
1. 先读 `ARCHITECTURE_REVIEW.md` 的"架构关注的 12 个维度"部分
2. 再读 `ARCHITECTURE_ISSUES.md` 了解具体技术债
3. 最后参考 `REFACTOR_PLAN.md` 学习改进方向

### 如果你准备修复问题
1. 读 `REFACTOR_PLAN.md` 的对应阶段（P0/P1/P2/P3）
2. 参考其中的代码示例和测试用例
3. 遵循 TDD 流程：先写失败测试，再修复

### 如果你负责 Code Review
1. 对照 `REFACTOR_PLAN.md` 的"审查清单"逐项验证
2. 关注 TDD 原则是否被遵守
3. 检查是否引入新的技术债

---

## 🔗 相关资源

- [Rust 官方文档](https://doc.rust-lang.org/)
- [Axum 框架](https://docs.rs/axum/)
- [Diesel ORM](https://diesel.rs/)
- [Tokio 异步运行时](https://tokio.rs/)

---

**最后更新**：2026-07-05
**维护人**：架构组
**审查周期**：每 2 周