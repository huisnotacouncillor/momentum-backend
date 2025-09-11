-- 回滚comments表功能扩展

-- 删除触发器
DROP TRIGGER IF EXISTS update_comments_updated_at ON comments;
DROP FUNCTION IF EXISTS update_updated_at_column();

-- 删除索引
DROP INDEX IF EXISTS idx_comments_parent_comment_id;
DROP INDEX IF EXISTS idx_comments_is_deleted;
DROP INDEX IF EXISTS idx_comment_mentions_comment_id;
DROP INDEX IF EXISTS idx_comment_mentions_mentioned_user_id;
DROP INDEX IF EXISTS idx_comment_attachments_comment_id;
DROP INDEX IF EXISTS idx_comment_reactions_comment_id;
DROP INDEX IF EXISTS idx_comment_reactions_user_id;

-- 删除新创建的表
DROP TABLE IF EXISTS comment_reactions;
DROP TABLE IF EXISTS comment_attachments;
DROP TABLE IF EXISTS comment_mentions;

-- 恢复comments表原始结构
ALTER TABLE comments RENAME COLUMN content TO body;

-- 删除新增的列
ALTER TABLE comments
DROP COLUMN IF EXISTS content_type,
DROP COLUMN IF EXISTS parent_comment_id,
DROP COLUMN IF EXISTS is_edited,
DROP COLUMN IF EXISTS is_deleted;