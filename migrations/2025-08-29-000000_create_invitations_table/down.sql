-- 删除 invitations 表
DROP TABLE IF EXISTS invitations;

-- 删除 invitation_status 枚举类型（仅在没有表使用它时）
DO $$
BEGIN
    -- 检查是否有表依赖于 invitation_status 枚举
    IF NOT EXISTS (
        SELECT 1
        FROM pg_depend
        WHERE refclassid = 'pg_type'::regclass AND
              refobjid = 'invitation_status'::regtype::oid
    ) THEN
        -- 如果没有依赖，则删除枚举
        DROP TYPE invitation_status;
    END IF;
END $$;