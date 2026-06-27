use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Comment models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::comments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Comment {
    pub id: Uuid,
    pub issue_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub content_type: Option<String>,
    pub parent_comment_id: Option<Uuid>,
    pub is_edited: Option<bool>,
    pub is_deleted: Option<bool>,
}

#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::comments)]
pub struct NewComment {
    pub issue_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub content_type: Option<String>,
    pub parent_comment_id: Option<Uuid>,
}

#[derive(AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::comments)]
pub struct UpdateComment {
    pub content: Option<String>,
    pub content_type: Option<String>,
    pub is_edited: Option<bool>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// Comment Mention models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::comment_mentions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CommentMention {
    pub id: Uuid,
    pub comment_id: Uuid,
    pub mentioned_user_id: Uuid,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::comment_mentions)]
pub struct NewCommentMention {
    pub comment_id: Uuid,
    pub mentioned_user_id: Uuid,
}

// Comment Attachment models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::comment_attachments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CommentAttachment {
    pub id: Uuid,
    pub comment_id: Uuid,
    pub file_name: String,
    pub file_url: String,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::comment_attachments)]
pub struct NewCommentAttachment {
    pub comment_id: Uuid,
    pub file_name: String,
    pub file_url: String,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
}

// Comment Reaction models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::comment_reactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CommentReaction {
    pub id: Uuid,
    pub comment_id: Uuid,
    pub user_id: Uuid,
    pub reaction_type: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::comment_reactions)]
pub struct NewCommentReaction {
    pub comment_id: Uuid,
    pub user_id: Uuid,
    pub reaction_type: String,
}

// API Response models
#[derive(Serialize, Deserialize)]
pub struct CommentWithDetails {
    #[serde(flatten)]
    pub comment: Comment,
    pub author: Option<crate::db::models::auth::User>,
    pub mentions: Vec<CommentMention>,
    pub attachments: Vec<CommentAttachment>,
    pub reactions: Vec<CommentReaction>,
    pub replies: Vec<CommentWithDetails>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub content_type: Option<String>,
    pub parent_comment_id: Option<Uuid>,
    pub mentions: Option<Vec<Uuid>>,
    pub attachments: Option<Vec<NewCommentAttachment>>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateCommentRequest {
    pub content: Option<String>,
    pub content_type: Option<String>,
}
