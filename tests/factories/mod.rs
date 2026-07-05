//! Test data factories
//!
//! Provides builders for creating test data in a consistent way

use uuid::Uuid;

/// Factory for creating test workspaces
#[allow(dead_code)]
pub struct WorkspaceFactory {
    name: String,
    url_key: String,
}

#[allow(dead_code)]
impl WorkspaceFactory {
    pub fn new() -> Self {
        Self {
            name: format!("Test Workspace {}", Uuid::new_v4().to_string().chars().take(8).collect::<String>()),
            url_key: format!("test-{}", Uuid::new_v4().to_string().replace("-", "").chars().take(8).collect::<String>()),
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn url_key(mut self, url_key: &str) -> Self {
        self.url_key = url_key.to_string();
        self
    }

    pub fn build(self) -> (String, String) {
        (self.name, self.url_key)
    }
}

impl Default for WorkspaceFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Factory for creating test teams
#[allow(dead_code)]
pub struct TeamFactory {
    name: String,
    team_key: String,
    description: Option<String>,
}

#[allow(dead_code)]
impl TeamFactory {
    pub fn new() -> Self {
        Self {
            name: format!("Test Team {}", Uuid::new_v4().to_string().chars().take(6).collect::<String>()),
            team_key: format!("TT-{}", Uuid::new_v4().to_string().replace("-", "").chars().take(4).collect::<String>()),
            description: None,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn team_key(mut self, key: &str) -> Self {
        self.team_key = key.to_string();
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn build(self) -> TeamData {
        TeamData {
            name: self.name,
            team_key: self.team_key,
            description: self.description,
        }
    }
}

impl Default for TeamFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Data structure for team creation
#[allow(dead_code)]
pub struct TeamData {
    pub name: String,
    pub team_key: String,
    pub description: Option<String>,
}

/// Factory for creating test users
#[allow(dead_code)]
pub struct UserFactory {
    email: String,
    username: String,
}

#[allow(dead_code)]
impl UserFactory {
    pub fn new() -> Self {
        let suffix = Uuid::new_v4().to_string().chars().take(8).collect::<String>();
        Self {
            email: format!("test_{}@example.com", suffix),
            username: format!("test_user_{}", suffix),
        }
    }

    pub fn email(mut self, email: &str) -> Self {
        self.email = email.to_string();
        self
    }

    pub fn username(mut self, username: &str) -> Self {
        self.username = username.to_string();
        self
    }

    pub fn build(self) -> UserData {
        UserData {
            email: self.email,
            username: self.username,
        }
    }
}

impl Default for UserFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Data structure for user data
#[allow(dead_code)]
pub struct UserData {
    pub email: String,
    pub username: String,
}

/// Factory for creating workflow status data
#[allow(dead_code)]
pub struct WorkflowStatusFactory {
    name: String,
    category: String,
    color: String,
    description: Option<String>,
    position: i32,
}

#[allow(dead_code)]
impl WorkflowStatusFactory {
    pub fn new() -> Self {
        Self {
            name: format!("Status {}", Uuid::new_v4().to_string().chars().take(4).collect::<String>()),
            category: "backlog".to_string(),
            color: "#4A90E2".to_string(),
            description: None,
            position: 0,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn category(mut self, category: &str) -> Self {
        self.category = category.to_string();
        self
    }

    pub fn color(mut self, color: &str) -> Self {
        self.color = color.to_string();
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn position(mut self, position: i32) -> Self {
        self.position = position;
        self
    }

    pub fn build(self) -> WorkflowStatusData {
        WorkflowStatusData {
            name: self.name,
            category: self.category,
            color: self.color,
            description: self.description,
            position: self.position,
        }
    }
}

impl Default for WorkflowStatusFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Data structure for workflow status
#[allow(dead_code)]
pub struct WorkflowStatusData {
    pub name: String,
    pub category: String,
    pub color: String,
    pub description: Option<String>,
    pub position: i32,
}

/// Factory for creating issue data
#[allow(dead_code)]
pub struct IssueFactory {
    title: String,
    description: Option<String>,
    priority: Option<String>,
}

#[allow(dead_code)]
impl IssueFactory {
    pub fn new() -> Self {
        Self {
            title: format!("Test Issue {}", Uuid::new_v4().to_string().chars().take(6).collect::<String>()),
            description: None,
            priority: Some("medium".to_string()),
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn priority(mut self, priority: &str) -> Self {
        self.priority = Some(priority.to_string());
        self
    }

    pub fn build(self) -> IssueData {
        IssueData {
            title: self.title,
            description: self.description,
            priority: self.priority,
        }
    }
}

impl Default for IssueFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Data structure for issue
#[allow(dead_code)]
pub struct IssueData {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
}

/// Factory for creating comment data
#[allow(dead_code)]
pub struct CommentFactory {
    content: String,
    content_type: Option<String>,
}

#[allow(dead_code)]
impl CommentFactory {
    pub fn new() -> Self {
        Self {
            content: format!("Test comment {}", Uuid::new_v4().to_string().chars().take(6).collect::<String>()),
            content_type: Some("markdown".to_string()),
        }
    }

    pub fn content(mut self, content: &str) -> Self {
        self.content = content.to_string();
        self
    }

    pub fn content_type(mut self, content_type: &str) -> Self {
        self.content_type = Some(content_type.to_string());
        self
    }

    pub fn build(self) -> CommentData {
        CommentData {
            content: self.content,
            content_type: self.content_type,
        }
    }
}

impl Default for CommentFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Data structure for comment
#[allow(dead_code)]
pub struct CommentData {
    pub content: String,
    pub content_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_factory() {
        let factory = WorkspaceFactory::new();
        let (name, url_key) = factory.build();
        assert!(name.starts_with("Test Workspace"));
        assert!(url_key.starts_with("test-"));
    }

    #[test]
    fn test_workspace_factory_custom_name() {
        let (name, url_key) = WorkspaceFactory::new()
            .name("Custom Workspace")
            .url_key("custom")
            .build();
        assert_eq!(name, "Custom Workspace");
        assert_eq!(url_key, "custom");
    }

    #[test]
    fn test_team_factory() {
        let data = TeamFactory::new().build();
        assert!(data.name.starts_with("Test Team"));
        assert!(data.team_key.starts_with("TT-"));
    }

    #[test]
    fn test_user_factory() {
        let data = UserFactory::new().build();
        assert!(data.email.contains("@example.com"));
        assert!(data.username.starts_with("test_user_"));
    }

    #[test]
    fn test_workflow_status_factory() {
        let data = WorkflowStatusFactory::new()
            .category("started")
            .position(1)
            .build();
        assert_eq!(data.category, "started");
        assert_eq!(data.position, 1);
    }

    #[test]
    fn test_issue_factory() {
        let data = IssueFactory::new()
            .title("Bug Report")
            .priority("high")
            .build();
        assert_eq!(data.title, "Bug Report");
        assert_eq!(data.priority, Some("high".to_string()));
    }

    #[test]
    fn test_comment_factory() {
        let data = CommentFactory::new()
            .content("This is a bug")
            .content_type("plain")
            .build();
        assert_eq!(data.content, "This is a bug");
        assert_eq!(data.content_type, Some("plain".to_string()));
    }
}
