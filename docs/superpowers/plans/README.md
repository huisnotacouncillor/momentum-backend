# 战略规划与决策材料（Plans）

> 12 篇规划文档生成于 **2026-06-27 集中 brainstorm**，覆盖产品定位、技术演进、运维工具链等议题。
> 本目录是 Momentum **"为什么这样做"** 的来源——新决策前先翻这里。

---

## 📚 文档索引

### 战略层

| 文档 | 摘要 | 落地状态 |
|---|---|---|
| [2026-06-27-momentum-product-planning.md](./2026-06-27-momentum-product-planning.md) | 产品总纲：定位"具身智能团队的研发 OS"，盘点现有代码 → 标注已实现/待新增 | 🟢 战略持续生效 |
| [2026-06-27-existing-backend.md](./2026-06-27-existing-backend.md) | 对 2026-06 时 backend 现状的盘点 | 🟢 历史价值，已被本目录其他文档吸收 |
| [2026-06-27-new-tables.md](./2026-06-27-new-tables.md) | 为扩展规划需要新增的 DB 表（plugin/agent_run 等） | 🟢 大部分已实现（plugin 系统落地），剩少量 v0.2 项 |

### 核心系统扩展

| 文档 | 摘要 | 落地状态 |
|---|---|---|
| [2026-06-27-issue-system.md](./2026-06-27-issue-system.md) | Issue 系统扩展：cycle/roadmap/plugin field 集成 | 🟢 核心完成（详见各 issue 路由） |
| [2026-06-27-diagnose-agent.md](./2026-06-27-diagnose-agent.md) | Diagnose Agent：AI 辅助问题排查 | 🟡 概念阶段，无实现 commit |
| [2026-06-27-mlops.md](./2026-06-27-mlops.md) | MLOps 集成（实验追踪、模型注册） | 🔴 战略储备，未启动 |
| [2026-06-27-simulation.md](./2026-06-27-simulation.md) | 仿真平台对接 | 🔴 战略储备，未启动 |

### 端 / 边 / 部署

| 文档 | 摘要 | 落地状态 |
|---|---|---|
| [2026-06-27-desktop-client.md](./2026-06-27-desktop-client.md) | Desktop 客户端规划 | 🔴 战略储备，未启动 |
| [2026-06-27-device-ota.md](./2026-06-27-device-ota.md) | 设备 OTA 通道 | 🔴 战略储备，未启动 |
| [2026-06-27-cicd-deploy.md](./2026-06-27-cicd-deploy.md) | CI/CD 与部署流水线 | 🟡 Dockerfile + docker-compose 已落地，CI 流程部分缺失 |
| [2026-06-27-mcp-platform.md](./2026-06-27-mcp-platform.md) | MCP 平台：把插件能力暴露给外部 AI 助手 | 🟡 通过 Integration 扩展点预留 |

---

## 🗺 状态图例

| 标识 | 含义 |
|---|---|
| 🟢 持续生效 | 文档决策已在代码中实现或正在持续演进 |
| 🟡 部分落地 | 核心思路实现，但仍有未完成的项（见各文档末尾"待办"） |
| 🔴 战略储备 | 概念/规划阶段，无实现 commit，可作为未来 roadmap 参考 |

---

## 📖 阅读建议

1. **新加入的开发者**：先读 `momentum-product-planning.md` 建立全局观，再按需翻其他
2. **做架构决策前**：先看 `existing-backend.md`（盘点）+ 相关专题文档，避免重复造轮子
3. **评估未来方向**：从 🔴 文档里挑，结合当前 backlog 判断优先级
4. **追溯历史决策**：所有"为什么这样设计"的问题，答案大概率在这 12 篇里

---

## ⚠️ 注意

- 这 12 篇**不是**实现计划（implementation plan），而是**决策材料**
- 实现计划应使用 [executing-plans skill](../../.claude/skills/executing-plans/SKILL.md) 产出，单独成文
- 文档日期均为 `2026-06-27`，但其中"已落地"标记基于该日状态；当前代码已变更的部分请以代码为准

---

**最后更新**：2026-07-12（建立索引）