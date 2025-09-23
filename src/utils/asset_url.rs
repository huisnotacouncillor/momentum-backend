use crate::config::AssetsConfig;
use std::borrow::Cow;

/// 通用的资源 URL 处理工具
#[derive(Clone, Debug)]
pub struct AssetUrlHelper {
    base_url: String,
    base_url_with_slash: String,
}

impl AssetUrlHelper {
    /// 创建新的 AssetUrlHelper 实例
    pub fn new(assets_config: &AssetsConfig) -> Self {
        let base_url = assets_config.base_url.clone();
        let base_url_with_slash = if base_url.ends_with('/') {
            base_url.clone()
        } else {
            format!("{}/", base_url)
        };

        Self {
            base_url,
            base_url_with_slash,
        }
    }

    /// 构建完整的资源 URL
    ///
    /// # 参数
    /// * `path` - 资源路径，可以是相对路径或绝对路径
    ///
    /// # 示例
    /// ```ignore
    /// let helper = AssetUrlHelper::new(&assets_config);
    /// let avatar_url = helper.build_url("avatars/user123.jpg");
    /// // 返回: "http://localhost:8000/assets/avatars/user123.jpg"
    /// ```ignore
    pub fn build_url(&self, path: &str) -> String {
        // 移除路径开头的斜杠（如果有的话）
        let clean_path = path.trim_start_matches('/');

        // 直接使用预计算的 base_url_with_slash，避免重复字符串操作
        format!("{}{}", self.base_url_with_slash, clean_path)
    }

    /// 构建用户头像 URL
    ///
    /// # 参数
    /// * `filename` - 头像文件名
    ///
    /// # 示例
    /// ```ignore
    /// let helper = AssetUrlHelper::new(&assets_config);
    /// let avatar_url = helper.build_avatar_url("user123.jpg");
    /// // 返回: "http://localhost:8000/assets/avatars/user123.jpg"
    /// ```ignore
    pub fn build_avatar_url(&self, filename: &str) -> String {
        self.build_url(&format!("avatars/{}", filename))
    }

    /// 构建团队图标 URL
    ///
    /// # 参数
    /// * `filename` - 图标文件名
    ///
    /// # 示例
    /// ```ignore
    /// let helper = AssetUrlHelper::new(&assets_config);
    /// let icon_url = helper.build_team_icon_url("team123.png");
    /// // 返回: "http://localhost:8000/assets/team-icons/team123.png"
    /// ```ignore
    pub fn build_team_icon_url(&self, filename: &str) -> String {
        self.build_url(&format!("team-icons/{}", filename))
    }

    /// 构建项目图标 URL
    ///
    /// # 参数
    /// * `filename` - 图标文件名
    ///
    /// # 示例
    /// ```ignore
    /// let helper = AssetUrlHelper::new(&assets_config);
    /// let icon_url = helper.build_project_icon_url("project123.png");
    /// // 返回: "http://localhost:8000/assets/project-icons/project123.png"
    /// ```ignore
    pub fn build_project_icon_url(&self, filename: &str) -> String {
        self.build_url(&format!("project-icons/{}", filename))
    }

    /// 构建附件 URL
    ///
    /// # 参数
    /// * `filename` - 附件文件名
    ///
    /// # 示例
    /// ```ignore
    /// let helper = AssetUrlHelper::new(&assets_config);
    /// let attachment_url = helper.build_attachment_url("document.pdf");
    /// // 返回: "http://localhost:8000/assets/attachments/document.pdf"
    /// ```ignore
    pub fn build_attachment_url(&self, filename: &str) -> String {
        self.build_url(&format!("attachments/{}", filename))
    }

    /// 检查 URL 是否为外部链接（不是基于当前 assets_url 的）
    ///
    /// # 参数
    /// * `url` - 要检查的 URL
    ///
    /// # 返回值
    /// * `true` - 如果是外部链接
    /// * `false` - 如果是内部资源路径
    pub fn is_external_url(&self, url: &str) -> bool {
        // 如果 URL 以 http:// 或 https:// 开头，且不包含当前 base_url，则为外部链接
        if url.starts_with("http://") || url.starts_with("https://") {
            !url.starts_with(&self.base_url)
        } else {
            // 相对路径，不是外部链接
            false
        }
    }

    /// 处理可能为相对路径的资源 URL
    /// 如果是外部链接，直接返回；如果是相对路径，则构建完整 URL
    ///
    /// # 参数
    /// * `url` - 资源 URL（可能是相对路径或外部链接）
    ///
    /// # 示例
    /// ```ignore
    /// let helper = AssetUrlHelper::new(&assets_config);
    ///
    /// // 外部链接，直接返回
    /// let external = helper.process_url("https://example.com/avatar.jpg");
    /// // 返回: "https://example.com/avatar.jpg"
    ///
    /// // 相对路径，构建完整 URL
    /// let internal = helper.process_url("avatars/user123.jpg");
    /// // 返回: "http://localhost:8000/assets/avatars/user123.jpg"
    /// ```
    pub fn process_url(&self, url: &str) -> String {
        if self.is_external_url(url) {
            url.to_string()
        } else {
            self.build_url(url)
        }
    }

    /// 优化的 URL 处理方法，避免不必要的字符串分配
    pub fn process_url_ref<'a>(&self, url: &'a str) -> Cow<'a, str> {
        if self.is_external_url(url) {
            Cow::Borrowed(url)
        } else {
            Cow::Owned(self.build_url(url))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_helper() -> AssetUrlHelper {
        let assets_config = AssetsConfig {
            base_url: "http://localhost:8000/assets".to_string(),
        };
        AssetUrlHelper::new(&assets_config)
    }

    #[test]
    fn test_build_url() {
        let helper = create_test_helper();

        assert_eq!(
            helper.build_url("avatars/user123.jpg"),
            "http://localhost:8000/assets/avatars/user123.jpg"
        );

        assert_eq!(
            helper.build_url("/avatars/user123.jpg"),
            "http://localhost:8000/assets/avatars/user123.jpg"
        );
    }

    #[test]
    fn test_build_avatar_url() {
        let helper = create_test_helper();

        assert_eq!(
            helper.build_avatar_url("user123.jpg"),
            "http://localhost:8000/assets/avatars/user123.jpg"
        );
    }

    #[test]
    fn test_build_team_icon_url() {
        let helper = create_test_helper();

        assert_eq!(
            helper.build_team_icon_url("team123.png"),
            "http://localhost:8000/assets/team-icons/team123.png"
        );
    }

    #[test]
    fn test_is_external_url() {
        let helper = create_test_helper();

        assert!(helper.is_external_url("https://example.com/avatar.jpg"));
        assert!(helper.is_external_url("http://external.com/image.png"));
        assert!(!helper.is_external_url("avatars/user123.jpg"));
        assert!(!helper.is_external_url("/avatars/user123.jpg"));
    }

    #[test]
    fn test_process_url() {
        let helper = create_test_helper();

        // 外部链接
        assert_eq!(
            helper.process_url("https://example.com/avatar.jpg"),
            "https://example.com/avatar.jpg"
        );

        // 内部路径
        assert_eq!(
            helper.process_url("avatars/user123.jpg"),
            "http://localhost:8000/assets/avatars/user123.jpg"
        );
    }
}
