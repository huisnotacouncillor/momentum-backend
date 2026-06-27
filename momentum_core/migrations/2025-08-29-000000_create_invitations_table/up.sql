-- 创建邀请状态枚举类型（如果不存在）
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'invitation_status') THEN
        CREATE TYPE invitation_status AS ENUM ('pending', 'accepted', 'declined', 'cancelled');
    END IF;
END $$;

-- 创建邀请表
CREATE TABLE invitations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL,
    role workspace_user_role NOT NULL DEFAULT 'member',
    status invitation_status NOT NULL DEFAULT 'pending',
    invited_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '7 days')
);

-- 添加索引以提高查询性能
CREATE INDEX idx_invitations_workspace_id ON invitations(workspace_id);
CREATE INDEX idx_invitations_email ON invitations(email);
CREATE INDEX idx_invitations_status ON invitations(status);
CREATE INDEX idx_invitations_expires_at ON invitations(expires_at);

-- 添加表注释
COMMENT ON TABLE invitations IS '工作区邀请表，用于管理工作区邀请';
COMMENT ON COLUMN invitations.id IS '邀请ID';
COMMENT ON COLUMN invitations.workspace_id IS '工作区ID';
COMMENT ON COLUMN invitations.email IS '被邀请人邮箱';
COMMENT ON COLUMN invitations.role IS '被邀请人在工作区中的角色';
COMMENT ON COLUMN invitations.status IS '邀请状态 (pending, accepted, declined, cancelled)';
COMMENT ON COLUMN invitations.invited_by IS '邀请人ID';
COMMENT ON COLUMN invitations.created_at IS '邀请创建时间';
COMMENT ON COLUMN invitations.updated_at IS '邀请更新时间';
COMMENT ON COLUMN invitations.expires_at IS '邀请过期时间';