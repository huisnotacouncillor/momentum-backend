-- Momentum Plugin System: 8 大扩展点 + 制品层 + 插件管理
-- 详见 docs/PLUGIN_SDK_DESIGN.md

-- ============================================================
-- 1. issue_field_definitions: 字段定义（插件注册）
-- ============================================================
CREATE TABLE issue_field_definitions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    plugin_id TEXT NOT NULL,
    field_key TEXT NOT NULL,
    label TEXT NOT NULL,
    field_type TEXT NOT NULL,                    -- 'text' | 'number' | 'enum' | 'date' | 'user' | 'bool'
    options JSONB,                               -- enum 选项 / 校验规则
    required BOOLEAN NOT NULL DEFAULT false,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workspace_id, plugin_id, field_key)
);

CREATE INDEX idx_issue_field_definitions_workspace ON issue_field_definitions(workspace_id);

-- ============================================================
-- 2. issue_field_values: 字段值
-- ============================================================
CREATE TABLE issue_field_values (
    issue_id UUID NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    field_id UUID NOT NULL REFERENCES issue_field_definitions(id) ON DELETE CASCADE,
    value JSONB NOT NULL,
    text_value TEXT,                              -- 反范式（搜索/过滤）
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (issue_id, field_id)
);

CREATE INDEX idx_issue_field_values_issue ON issue_field_values(issue_id);
CREATE INDEX idx_issue_field_values_text ON issue_field_values(text_value);

-- ============================================================
-- 3. plugins: 插件元数据
-- ============================================================
CREATE TABLE plugins (
    id TEXT PRIMARY KEY,                          -- 'embodied-intelligence'
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    publisher TEXT,
    manifest JSONB NOT NULL,                      -- 完整 manifest
    status TEXT NOT NULL DEFAULT 'available',     -- 'available' | 'deprecated'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- 4. plugin_installations: 插件安装
-- ============================================================
CREATE TABLE plugin_installations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'installed',     -- 'installed' | 'enabled' | 'disabled' | 'error'
    installed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    enabled_at TIMESTAMPTZ,
    error_message TEXT,
    UNIQUE(workspace_id, plugin_id)
);

CREATE INDEX idx_plugin_installations_workspace ON plugin_installations(workspace_id);
CREATE INDEX idx_plugin_installations_status ON plugin_installations(workspace_id, status);

-- ============================================================
-- 5. plugin_storage: 插件隔离存储
-- ============================================================
CREATE TABLE plugin_storage (
    plugin_id TEXT NOT NULL,
    workspace_id UUID NOT NULL,
    namespace TEXT NOT NULL,
    key TEXT NOT NULL,
    value JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (plugin_id, workspace_id, namespace, key)
);

CREATE INDEX idx_plugin_storage_plugin_workspace ON plugin_storage(plugin_id, workspace_id);

-- ============================================================
-- 6. plugin_audit: 插件审计日志
-- ============================================================
CREATE TABLE plugin_audit (
    id BIGSERIAL PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    workspace_id UUID,
    event TEXT NOT NULL,
    payload JSONB,
    actor_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_plugin_audit_plugin ON plugin_audit(plugin_id, created_at DESC);
CREATE INDEX idx_plugin_audit_workspace ON plugin_audit(workspace_id, created_at DESC);

-- ============================================================
-- 7. agent_runs: Agent 执行记录
-- ============================================================
CREATE TABLE agent_runs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    issue_id UUID REFERENCES issues(id) ON DELETE SET NULL,
    plugin_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',       -- 'pending' | 'running' | 'succeeded' | 'failed' | 'cancelled'
    input JSONB,
    output JSONB,
    error TEXT,
    tokens_input INTEGER,
    tokens_output INTEGER,
    duration_ms INTEGER,
    actor_id UUID,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_agent_runs_workspace ON agent_runs(workspace_id, started_at DESC);
CREATE INDEX idx_agent_runs_issue ON agent_runs(issue_id);

-- ============================================================
-- 8. outbox: 事件发件箱（插件 publish_event 写入）
-- ============================================================
CREATE TABLE outbox (
    id BIGSERIAL PRIMARY KEY,
    aggregate_type TEXT,
    aggregate_id UUID,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at TIMESTAMPTZ,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    next_retry_at TIMESTAMPTZ
);

CREATE INDEX idx_outbox_pending ON outbox(id) WHERE delivered_at IS NULL;
CREATE INDEX idx_outbox_aggregate ON outbox(aggregate_type, aggregate_id, id);

-- ============================================================
-- 9. issues.version: 乐观锁 + 同步游标
-- ============================================================
ALTER TABLE issues ADD COLUMN version INTEGER NOT NULL DEFAULT 1;