-- 自动化规则表
CREATE TABLE automation_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    team_id UUID REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    trigger_type VARCHAR(50) NOT NULL,
    trigger_config JSONB,
    conditions JSONB NOT NULL DEFAULT '[]',
    actions JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_automation_rules_workspace ON automation_rules(workspace_id);
CREATE INDEX idx_automation_rules_team ON automation_rules(team_id);
CREATE INDEX idx_automation_rules_enabled ON automation_rules(is_enabled);
CREATE INDEX idx_automation_rules_trigger ON automation_rules(trigger_type);
