-- 创建用户在工作区中的角色枚举类型（如果不存在）
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'workspace_user_role') THEN
        CREATE TYPE workspace_user_role AS ENUM ('owner', 'admin', 'member', 'guest');
    END IF;
END $$;

-- 检查是否存在旧表，如果存在则重命名；否则创建新表
DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'workspace_users') THEN
        ALTER TABLE workspace_users RENAME TO workspace_members;
        -- 更新关联索引名称
        ALTER INDEX IF EXISTS workspace_users_pkey RENAME TO workspace_members_pkey;
        ALTER INDEX IF EXISTS idx_workspace_users_workspace_id RENAME TO idx_workspace_members_workspace_id;
        ALTER INDEX IF EXISTS idx_workspace_users_user_id RENAME TO idx_workspace_members_user_id;
        ALTER INDEX IF EXISTS idx_workspace_users_role RENAME TO idx_workspace_members_role;
    ELSE
        -- 创建工作区成员表
        CREATE TABLE workspace_members (
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
            role workspace_user_role NOT NULL DEFAULT 'member',
            created_at TIMESTAMPTz NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTz NOT NULL DEFAULT NOW(),
            PRIMARY KEY (user_id, workspace_id)
        );

        -- 添加索引以提高查询性能
        CREATE INDEX idx_workspace_members_workspace_id ON workspace_members(workspace_id);
        CREATE INDEX idx_workspace_members_user_id ON workspace_members(user_id);
        CREATE INDEX idx_workspace_members_role ON workspace_members(role);
        
        -- 添加表注释
        COMMENT ON TABLE workspace_members IS '工作区成员表，用于管理工作区中用户的权限角色';
        COMMENT ON COLUMN workspace_members.user_id IS '用户ID';
        COMMENT ON COLUMN workspace_members.workspace_id IS '工作区ID';
        COMMENT ON COLUMN workspace_members.role IS '用户在工作区中的角色';
        COMMENT ON COLUMN workspace_members.created_at IS '记录创建时间';
        COMMENT ON COLUMN workspace_members.updated_at IS '记录更新时间';
    END IF;
END $$;