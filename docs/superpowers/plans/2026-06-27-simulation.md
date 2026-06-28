# 仿真集成实现计划

> **目标：** 在现有 Axum API 基础上，新增仿真场景管理 API 和仿真任务追踪（sim_scenes/sim_runs 表已在 P0-D1 新增）
>
> **技术栈：** Rust/Axum, diesel, gRPC (tonic), S3
>
> **注意：** 仿真 Agent（Sim Agent）在 `momentum_plugin_host` 里通过 gRPC 调用外部仿真器

---

## 任务 1：仿真场景管理

**文件：**
- 创建：`momentum_core/migrations/YYYYMMDD_create_sim_tables/up.sql`
- 创建：`apps/api-gateway/src/graphql/schema/simulation.graphql`

- [ ] **步骤 1：创建仿真相关表**

```sql
CREATE TABLE sim_scenes (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  name VARCHAR(100) NOT NULL,
  description TEXT,
  environment VARCHAR(50) NOT NULL CHECK (environment IN ('isaac_sim', 'gazebo', 'mujoco', 'carla', 'custom')),
  config JSONB DEFAULT '{}',
  artifact_id UUID REFERENCES artifacts(id),  -- 场景配置文件
  created_by UUID REFERENCES users(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE sim_runs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  issue_id UUID REFERENCES issues(id) ON DELETE SET NULL,
  scene_id UUID NOT NULL REFERENCES sim_scenes(id),
  status VARCHAR(20) NOT NULL DEFAULT 'queued'
    CHECK (status IN ('queued', 'running', 'completed', 'failed')),
  artifact_id UUID REFERENCES artifacts(id),  -- 仿真报告 artifact
  metrics JSONB DEFAULT '{}',
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sim_runs_issue ON sim_runs(issue_id);
CREATE INDEX idx_sim_runs_scene ON sim_runs(scene_id);
```

- [ ] **步骤 2：Commit**

```bash
git add momentum_core/migrations/YYYYMMDD_create_sim_tables/
git add apps/api-gateway/src/graphql/schema/simulation.graphql
git commit -m "feat(sim): add sim_scenes and sim_runs tables"
```

---

**计划已完成并保存到 `docs/superpowers/plans/2026-06-27-simulation.md`**
