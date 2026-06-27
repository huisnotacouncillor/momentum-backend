//! Core utilities - pure Rust, no HTTP dependencies
//!
//! This module contains core utilities that don't depend on any web frameworks.

use serde::{Deserialize, Serialize};

/// Asset URL helper configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetConfig {
    pub base_url: String,
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3000/static".to_string(),
        }
    }
}

/// Asset URL helper for building resource URLs
///
/// This is a simplified version that works without HTTP dependencies.
/// The full implementation with external URL detection lives in momentum_api.
#[derive(Debug, Clone)]
pub struct AssetUrlHelper {
    config: AssetConfig,
}

impl AssetUrlHelper {
    pub fn new(config: &AssetConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Build a URL for an asset path
    pub fn build_url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}/{}", self.config.base_url.trim_end_matches('/'), path)
        }
    }

    /// Build avatar URL
    pub fn build_avatar_url(&self, filename: &str) -> String {
        self.build_url(&format!("avatars/{}", filename))
    }

    /// Build team icon URL
    pub fn build_team_icon_url(&self, filename: &str) -> String {
        self.build_url(&format!("team-icons/{}", filename))
    }

    /// Build project icon URL
    pub fn build_project_icon_url(&self, filename: &str) -> String {
        self.build_url(&format!("project-icons/{}", filename))
    }

    /// Build attachment URL
    pub fn build_attachment_url(&self, filename: &str) -> String {
        self.build_url(&format!("attachments/{}", filename))
    }

    /// Process a URL - if it's already external, return as-is
    pub fn process_url(&self, url: &str) -> String {
        self.build_url(url)
    }

    /// Process a URL and return as Cow to avoid unnecessary allocation
    pub fn process_url_ref<'a>(&self, url: &'a str) -> std::borrow::Cow<'a, str> {
        if self.is_external_url(url) {
            std::borrow::Cow::Borrowed(url)
        } else {
            std::borrow::Cow::Owned(self.build_url(url))
        }
    }

    /// Check if URL is external (starts with http:// or https://)
    pub fn is_external_url(&self, url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }
}

impl Default for AssetUrlHelper {
    fn default() -> Self {
        Self::new(&AssetConfig::default())
    }
}
