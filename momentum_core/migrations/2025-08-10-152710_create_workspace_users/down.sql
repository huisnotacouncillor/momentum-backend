-- 检查是否存在新表，如果存在则重命名回原来的名字
DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'workspace_members') THEN
        -- 重命名表
        ALTER TABLE workspace_members RENAME TO workspace_users;

        -- 恢复关联索引名称
        ALTER INDEX IF EXISTS workspace_members_pkey RENAME TO workspace_users_pkey;
        ALTER INDEX IF EXISTS idx_workspace_members_workspace_id RENAME TO idx_workspace_users_workspace_id;
        ALTER INDEX IF EXISTS idx_workspace_members_user_id RENAME TO idx_workspace_users_user_id;
        ALTER INDEX IF EXISTS idx_workspace_members_role RENAME TO idx_workspace_users_role;
    END IF;
END $$;

-- 删除workspace_user_role枚举类型（仅在没有表使用它时）
-- 注意：由于可能还有其他表使用此枚举，实际上我们不会删除它
-- 如果确实需要删除，应该在确保没有依赖的情况下进行，并考虑使用 CASCADE 选项
DO $$
BEGIN
    -- 检查是否有表依赖于 workspace_user_role 枚举
    IF NOT EXISTS (
        SELECT 1
        FROM pg_depend
        WHERE refclassid = 'pg_type'::regclass AND
              refobjid = 'workspace_user_role'::regtype::oid
    ) THEN
        -- 如果没有依赖，则删除枚举
        DROP TYPE workspace_user_role;
    END IF;
END $$;