-- 启用UUID扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 扩展comments表功能
ALTER TABLE comments
ADD COLUMN content_type VARCHAR(20) DEFAULT 'markdown',
ADD COLUMN parent_comment_id UUID REFERENCES comments(id) ON DELETE CASCADE,
ADD COLUMN is_edited BOOLEAN DEFAULT FALSE,
ADD COLUMN is_deleted BOOLEAN DEFAULT FALSE;

-- 重命名body字段为content，保持一致性
ALTER TABLE comments RENAME COLUMN body TO content;

-- 创建comment_mentions表用于@提及功能
CREATE TABLE comment_mentions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    comment_id UUID NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    mentioned_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 创建comment_attachments表用于文件附件
CREATE TABLE comment_attachments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    comment_id UUID NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    file_name VARCHAR(255) NOT NULL,
    file_url TEXT NOT NULL,
    file_size BIGINT,
    mime_type VARCHAR(100),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 创建comment_reactions表用于表情反应
CREATE TABLE comment_reactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    comment_id UUID NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reaction_type VARCHAR(50) NOT NULL, -- 'like', 'heart', 'thumbs_up', etc.
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(comment_id, user_id, reaction_type)
);

-- 添加索引优化查询性能
CREATE INDEX idx_comments_parent_comment_id ON comments(parent_comment_id);
CREATE INDEX idx_comments_is_deleted ON comments(is_deleted);
CREATE INDEX idx_comment_mentions_comment_id ON comment_mentions(comment_id);
CREATE INDEX idx_comment_mentions_mentioned_user_id ON comment_mentions(mentioned_user_id);
CREATE INDEX idx_comment_attachments_comment_id ON comment_attachments(comment_id);
CREATE INDEX idx_comment_reactions_comment_id ON comment_reactions(comment_id);
CREATE INDEX idx_comment_reactions_user_id ON comment_reactions(user_id);

-- 添加触发器自动更新updated_at字段（如果不存在）
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

DROP TRIGGER IF EXISTS update_comments_updated_at ON comments;
CREATE TRIGGER update_comments_updated_at
    BEFORE UPDATE ON comments
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();