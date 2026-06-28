# 新增数据表实现计划

> **面向 AI 代理的工作者：** 本计划基于现有 `momentum_core` Diesel 迁移格式编写。
> 目标：在现有 37 张表基础上新增 `artifacts` / `devices` / `fleets` / `model_versions` / `sim_scenes` / `sim_runs` 表。
>
> **文件结构遵循**：`momentum_core/migrations/YYYYMMDD-HHMMSS_description/` + `up.sql` / `down.sql`

---

## 文件结构

```
momentum_core/
└── migrations/
    ├── 2026-06-27-000001_add_artifacts/          # 制品关联（本计划）
    ├── 2026-06-27-000002_add_devices_and_fleets/     # 设备管理（本计划）
    └── 2026-06-27-000003_add_model_versions/       # 模型注册表（本计划）
```

---

## 任务 1：artifacts 表（跨学科制品关联）

### 文件
- 创建：`momentum_core/migrations/2026-06-27-000001_add_artifacts/up.sql`
- 创建：`momentum_core/migrations/2026-06-27-000001_add_artifacts/down.sql`
- 创建：`momentum_core/src/db/models/artifact.rs`
- 修改：`momentum_core/src/db/models/mod.rs`
- 创建：`momentum_core/src/db/repositories/artifacts.rs`
- 修改：`momentum_core/src/db/repositories/mod.rs`

- [ ] **步骤 1：创建 up.sql**

```sql
-- ============================================================
-- artifacts: 关联 Issue 与各类外部制品（跨学科协作核心）
-- ============================================================

CREATE TABLE artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- 归属
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,

    -- 制品类型
    type VARCHAR(30) NOT NULL CHECK (type IN (
        'code',           -- 代码文件
        'pr',             -- GitHub PR
        'design',         -- Figma 设计稿
        'model',          -- 模型权重文件
        'dataset',        -- 训练数据集
        'cad',            -- CAD 模型
        'firmware',       -- 固件
        'sim_report',     -- 仿真报告
        'test_report',    -- 测试报告
        'deploy'           -- 部署记录
    )),

    -- 外部引用 ID
    ref_id VARCHAR(255) NOT NULL,

    -- 可访问地址
    url TEXT,

    -- 制品元数据（JSONB）
    -- pr: { "repo": "owner/repo", "number": 123, "author": "..." }
    -- design: { "file_key": "abc123", "version": "2.0" }
    -- model: { "framework": "pytorch", "size_mb": 485 }
    metadata JSONB DEFAULT '{}',

    -- 关联 Issue
    linked_issue_id UUID NOT NULL REFERENCES issues(id) ON DELETE CASCADE,

    -- 语义化版本
    version VARCHAR(50) NOT NULL DEFAULT '1.0.0',

    -- 部署类制品关联设备
    device_id UUID REFERENCES devices(id) ON DELETE SET NULL,

    -- 审计
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引
CREATE INDEX idx_artifacts_workspace ON artifacts(workspace_id);
CREATE INDEX idx_artifacts_issue ON artifacts(linked_issue_id);
CREATE INDEX idx_artifacts_type ON artifacts(type);
CREATE INDEX idx_artifacts_device ON artifacts(device_id) WHERE device_id IS NOT NULL;
CREATE INDEX idx_artifacts_ref_id ON artifacts(ref_id);
CREATE INDEX idx_artifacts_created ON artifacts(created_at DESC);
```

- [ ] **步骤 2：创建 down.sql**

```sql
DROP TABLE IF EXISTS artifacts;
```

- [ ] **步骤 3：创建 artifact.rs**

```rust
use crate::schema::artifacts;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = artifacts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Artifact {
    pub id: Uuid,
    pub workspace_id: Uuid,
    #[serde(rename = "type")]
    pub artifact_type: String,
    pub ref_id: String,
    pub url: Option<String>,
    pub metadata: serde_json::Value,
    pub linked_issue_id: Uuid,
    pub version: String,
    pub device_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = artifacts)]
pub struct NewArtifact<'a> {
    pub workspace_id: Uuid,
    pub artifact_type: &'a str,
    pub ref_id: &'a str,
    pub url: Option<&'a str>,
    pub metadata: Option<serde_json::Value>,
    pub linked_issue_id: Uuid,
    pub version: Option<&'a str>,
    pub device_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = artifacts)]
pub struct UpdateArtifact {
    pub metadata: Option<serde_json::Value>,
    pub version: Option<String>,
    pub url: Option<String>,
}
```

- [ ] **步骤 4：修改 models/mod.rs**

```rust
pub mod artifact;  // 新增
```

- [ ] **步骤 5：创建 artifacts.rs repository**

```rust
use diesel::prelude::*;
use uuid::Uuid;
use crate::db::models::artifact::{Artifact, NewArtifact, UpdateArtifact};
use crate::schema::artifacts;

pub struct ArtifactRepo;

impl ArtifactRepo {
    pub fn list_by_issue(
        conn: &mut PgConnection,
        issue_id: Uuid,
    ) -> Result<Vec<Artifact>, diesel::result::Error> {
        artifacts::table
            .filter(artifacts::linked_issue_id.eq(issue_id))
            .order(artifacts::created_at.desc())
            .load(conn)
    }

    pub fn list_by_issue_type(
        conn: &mut PgConnection,
        issue_id: Uuid,
        artifact_type: &str,
    ) -> Result<Vec<Artifact>, diesel::result::Error> {
        artifacts::table
            .filter(artifacts::linked_issue_id.eq(issue_id))
            .filter(artifacts::artifact_type.eq(artifact_type))
            .load(conn)
    }

    pub fn insert(
        conn: &mut PgConnection,
        new: &NewArtifact,
    ) -> Result<Artifact, diesel::result::Error> {
        diesel::insert_into(artifacts::table)
            .values(new)
            .get_result(conn)
    }

    pub fn update(
        conn: &mut PgConnection,
        id: Uuid,
        changes: UpdateArtifact,
    ) -> Result<Artifact, diesel::result::Error> {
        diesel::update(artifacts::table.filter(artifacts::id.eq(id)))
            .set(&changes)
            .get_result(conn)
    }

    pub fn delete(conn: &mut PgConnection, id: Uuid) -> Result<usize, diesel::result::Error> {
        diesel::delete(artifacts::table.filter(artifacts::id.eq(id)))
            .execute(conn)
    }
}
```

- [ ] **步骤 6：修改 repositories/mod.rs**

```rust
pub mod artifacts;  // 新增
```

- [ ] **步骤 7：运行迁移验证**

```bash
cd momentum_core
export DATABASE_URL=postgres://postgres:postgres@localhost:5434/rust-backend
diesel migration run --migration-dir migrations
# 预期：Running migration 2026-06-27-000001_add_artifacts
```

- [ ] **步骤 8：Commit**

```bash
git add momentum_core/migrations/2026-06-27-000001_add_artifacts/
git add momentum_core/src/db/models/artifact.rs momentum_core/src/db/models/mod.rs
git add momentum_core/src/db/repositories/artifacts.rs momentum_core/src/db/repositories/mod.rs
git commit -m "feat(db: add artifacts table for cross-discipline artifact linking"
```

---

## 任务 2：devices / fleets 表（设备管理）

### 文件
- 创建：`momentum_core/migrations/2026-06-27-000002_add_devices_and_fleets/up.sql`
- 创建：`momentum_core/migrations/2026-06-27-000002_add_devices_and_fleets/down.sql`
- 创建：`momentum_core/src/db/models/device.rs`
- 创建：`momentum_core/src/db/models/firmware.rs`
- 修改：`momentum_core/src/db/models/mod.rs`
- 创建：`momentum_core/src/db/repositories/devices.rs`
- 创建：`momentum_core/src/db/repositories/firmware.rs`
- 修改：`momentum_core/src/db/repositories/mod.rs`

- [ ] **步骤 1：创建 up.sql**

```sql
-- ============================================================
-- fleets: 设备分组
-- ============================================================
CREATE TABLE fleets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fleets_workspace ON fleets(workspace_id);

-- ============================================================
-- devices: 具身智能设备（机器人、传感器、边缘计算设备等）
-- ============================================================
CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    fleet_id UUID REFERENCES fleets(id) ON DELETE SET NULL,

    -- 基本信息
    name VARCHAR(100) NOT NULL,
    serial_number VARCHAR(100) UNIQUE,
    device_type VARCHAR(50) NOT NULL CHECK (device_type IN (
        'robot', 'sensor', 'edge_gpu', 'vehicle', 'drone', 'other'
    )),

    -- 版本信息
    hardware_version VARCHAR(50) NOT NULL DEFAULT 'v1.0.0',
    firmware_version VARCHAR(50),
    software_version VARCHAR(50),

    -- 当前运行版本
    model_artifact_id UUID REFERENCES artifacts(id) ON DELETE SET NULL,

    -- 状态
    status VARCHAR(20) NOT NULL DEFAULT 'offline' CHECK (status IN (
        'online', 'offline', 'error', 'maintenance', 'decommissioned'
    )),

    -- 位置
    location VARCHAR(200),

    -- 遥测（最新快照）
    telemetry JSONB DEFAULT '{}',

    -- 最后在线
    last_seen_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_devices_workspace ON devices(workspace_id);
CREATE INDEX idx_devices_fleet ON devices(fleet_id) WHERE fleet_id IS NOT NULL;
CREATE INDEX idx_devices_status ON devices(status);
CREATE INDEX idx_devices_serial ON devices(serial_number) WHERE serial_number IS NOT NULL;
CREATE INDEX idx_devices_last_seen ON devices(last_seen_at);

-- ============================================================
-- firmware_versions: 固件版本注册
-- ============================================================
CREATE TABLE firmware_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_type VARCHAR(50) NOT NULL,
    version VARCHAR(50) NOT NULL,
    artifact_id UUID REFERENCES artifacts(id) ON DELETE SET NULL,
    changelog TEXT,
    is_stable BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(device_type, version)
);

CREATE INDEX idx_firmware_device_type ON firmware_versions(device_type);
CREATE INDEX idx_firmware_stable ON firmware_versions(device_type, is_stable) WHERE is_stable = true;

-- ============================================================
-- device_firmware_history: 固件升级历史
-- ============================================================
CREATE TABLE device_firmware_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    from_version VARCHAR(50),
    to_version VARCHAR(50) NOT NULL,
    deployed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deployed_by UUID REFERENCES users(id) ON DELETE SET NULL,
    result VARCHAR(20) DEFAULT 'success' CHECK (result IN ('success', 'failed', 'rollback'))
);

CREATE INDEX idx_firmware_history_device ON device_firmware_history(device_id);
CREATE INDEX idx_firmware_history_time ON device_firmware_history(deployed_at DESC);
```

- [ ] **步骤 2：创建 down.sql**

```sql
DROP TABLE IF EXISTS device_firmware_history;
DROP TABLE IF EXISTS firmware_versions;
DROP TABLE IF EXISTS devices;
DROP TABLE IF EXISTS fleets;
```

- [ ] **步骤 3：创建 device.rs**

```rust
use crate::schema::{devices, firmware_versions, device_firmware_history};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = devices)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Device {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub fleet_id: Option<Uuid>,
    pub name: String,
    pub serial_number: Option<String>,
    pub device_type: String,
    pub hardware_version: String,
    pub firmware_version: Option<String>,
    pub software_version: Option<String>,
    pub model_artifact_id: Option<Uuid>,
    pub status: String,
    pub location: Option<String>,
    pub telemetry: serde_json::Value,
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = devices)]
pub struct NewDevice<'a> {
    pub workspace_id: Uuid,
    pub fleet_id: Option<Uuid>,
    pub name: &'a str,
    pub serial_number: Option<&'a str>,
    pub device_type: &'a str,
    pub hardware_version: Option<&'a str>,
    pub firmware_version: Option<&'a str>,
    pub software_version: Option<&'a str>,
    pub model_artifact_id: Option<Uuid>,
    pub status: Option<&'a str>,
    pub location: Option<&'a str>,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = devices)]
pub struct UpdateDevice {
    pub name: Option<String>,
    pub hardware_version: Option<String>,
    pub firmware_version: Option<String>,
    pub software_version: Option<String>,
    pub model_artifact_id: Option<Uuid>,
    pub status: Option<String>,
    pub location: Option<String>,
    pub telemetry: Option<serde_json::Value>,
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
}
```

- [ ] **步骤 4：Commit**

```bash
git add momentum_core/migrations/2026-06-27-000002_add_devices_and_fleets/
git add momentum_core/src/db/models/device.rs momentum_core/src/db/models/firmware.rs momentum_core/src/db/models/mod.rs
git add momentum_core/src/db/repositories/devices.rs momentum_core/src/db/repositories/firmware.rs momentum_core/src/db/repositories/mod.rs
git commit -m "feat(db): add devices, fleets, firmware_versions tables"
```

---

## 任务 3：model_versions / sim_scenes / sim_runs 表

### 文件
- 创建：`momentum_core/migrations/2026-06-27-000003_add_model_versions/up.sql`
- 创建：`momentum_core/migrations/2026-06-27-000003_add_model_versions/down.sql`
- 创建：`momentum_core/src/db/models/model_version.rs`
- 创建：`momentum_core/src/db/models/sim_scene.rs`
- 修改：`momentum_core/src/db/models/mod.rs`
- 创建：`momentum_core/src/db/repositories/model_versions.rs`
- 创建：`momentum_core/src/db/repositories/sim_scenes.rs`
- 修改：`momentum_core/src/db/repositories/mod.rs`

- [ ] **步骤 1：创建 up.sql**

```sql
-- ============================================================
-- model_versions: 模型注册表（MLOps 核心）
-- ============================================================
CREATE TABLE model_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,

    name VARCHAR(100) NOT NULL,
    version VARCHAR(50) NOT NULL,
    description TEXT,

    -- 制品
    artifact_id UUID REFERENCES artifacts(id) ON DELETE SET NULL,

    -- 血统
    trained_on_dataset_id UUID REFERENCES artifacts(id) ON DELETE SET NULL,
    training_code_artifact_id UUID REFERENCES artifacts(id) ON DELETE SET NULL,
    training_config JSONB DEFAULT '{}',

    -- 评估指标
    metrics JSONB DEFAULT '{}',

    -- 上游
    parent_model_version_id UUID REFERENCES model_versions(id) ON DELETE SET NULL,

    -- 部署状态
    is_production BOOLEAN DEFAULT false,
    production_deployment_id UUID REFERENCES deployments(id) ON DELETE SET NULL,

    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(workspace_id, name, version)
);

CREATE INDEX idx_model_versions_workspace ON model_versions(workspace_id);
CREATE INDEX idx_model_versions_name ON model_versions(workspace_id, name);
CREATE INDEX idx_model_versions_production ON model_versions(workspace_id, is_production) WHERE is_production = true;
CREATE INDEX idx_model_versions_dataset ON model_versions(trained_on_dataset_id) WHERE trained_on_dataset_id IS NOT NULL;

-- ============================================================
-- deployments: 部署记录（云端 + 边缘）
-- ============================================================
CREATE TABLE deployments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,

    -- 关联
    issue_id UUID REFERENCES issues(id) ON DELETE SET NULL,
    artifact_id UUID REFERENCES artifacts(id) ON DELETE SET NULL,
    device_id UUID REFERENCES devices(id) ON DELETE SET NULL,

    -- 版本
    version VARCHAR(50) NOT NULL,

    -- 策略
    environment VARCHAR(20) NOT NULL CHECK (environment IN ('dev', 'staging', 'production', 'edge')),
    strategy VARCHAR(20) NOT NULL DEFAULT 'immediate' CHECK (strategy IN ('immediate', 'canary', 'percentage', 'geo')),

    -- 状态
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled', 'rolled_back')),

    -- 进度
    target_devices INT DEFAULT 0,
    succeeded_devices INT DEFAULT 0,
    failed_devices INT DEFAULT 0,

    -- 回滚
    rolled_back_from UUID REFERENCES deployments(id) ON DELETE SET NULL,

    -- 审计
    triggered_by UUID REFERENCES users(id) ON DELETE SET NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_deployments_workspace ON deployments(workspace_id);
CREATE INDEX idx_deployments_issue ON deployments(issue_id) WHERE issue_id IS NOT NULL;
CREATE INDEX idx_deployments_device ON deployments(device_id) WHERE device_id IS NOT NULL;
CREATE INDEX idx_deployments_status ON deployments(status);
CREATE INDEX idx_deployments_environment ON deployments(environment);

-- ============================================================
-- sim_scenes: 仿真场景库
-- ============================================================
CREATE TABLE sim_scenes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    environment VARCHAR(50) NOT NULL CHECK (environment IN ('isaac_sim', 'gazebo', 'mujoco', 'carla', 'custom')),
    config JSONB DEFAULT '{}',
    artifact_id UUID REFERENCES artifacts(id) ON DELETE SET NULL,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sim_scenes_workspace ON sim_scenes(workspace_id);
CREATE INDEX idx_sim_scenes_env ON sim_scenes(environment);

-- ============================================================
-- sim_runs: 仿真运行记录
-- ============================================================
CREATE TABLE sim_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,

    issue_id UUID REFERENCES issues(id) ON DELETE SET NULL,
    scene_id UUID NOT NULL REFERENCES sim_scenes(id),

    status VARCHAR(20) NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'running', 'completed', 'failed')),

    -- 仿真报告 artifact
    report_artifact_id UUID REFERENCES artifacts(id) ON DELETE SET NULL,

    -- 关键指标
    metrics JSONB DEFAULT '{}',

    -- 时间
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sim_runs_issue ON sim_runs(issue_id) WHERE issue_id IS NOT NULL;
CREATE INDEX idx_sim_runs_scene ON sim_runs(scene_id);
CREATE INDEX idx_sim_runs_status ON sim_runs(status);
```

- [ ] **步骤 2：Commit**

```bash
git add momentum_core/migrations/2026-06-27-000003_add_model_versions/
git add momentum_core/src/db/models/model_version.rs momentum_core/src/db/models/sim_scene.rs momentum_core/src/db/models/mod.rs
git add momentum_core/src/db/repositories/model_versions.rs momentum_core/src/db/repositories/sim_scenes.rs momentum_core/src/db/repositories/mod.rs
git commit -m "feat(db): add model_versions, deployments, sim_scenes, sim_runs tables"
```

---

## 自检

1. [ ] artifacts 表迁移运行成功
2. [ ] devices/fleets/firmware 表迁移运行成功
3. [ ] model_versions/deployments/sim_* 表迁移运行成功
4. [ ] Diesel schema 自动重新生成 (`diesel print-schema`)
5. [ ] `cargo check --workspace` 通过

---

## 执行选项

**计划已完成并保存到 `docs/superpowers/plans/2026-06-27-new-tables.md`**
