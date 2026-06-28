-- 添加 tsvector 列
ALTER TABLE issues ADD COLUMN search_vector tsvector;

-- 创建 GIN 索引
CREATE INDEX idx_issues_search_vector ON issues USING GIN(search_vector);

-- 创建更新触发器函数
CREATE OR REPLACE FUNCTION update_issue_search_vector()
RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.description, '')), 'B');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 创建触发器
CREATE TRIGGER trigger_update_issue_search_vector
    BEFORE INSERT OR UPDATE OF title, description ON issues
    FOR EACH ROW
    EXECUTE FUNCTION update_issue_search_vector();

-- 更新现有数据的 search_vector
UPDATE issues SET search_vector =
    setweight(to_tsvector('english', COALESCE(title, '')), 'A') ||
    setweight(to_tsvector('english', COALESCE(description, '')), 'B');