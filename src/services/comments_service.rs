use diesel::prelude::*;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    db::models::comment::{Comment, NewComment},
    db::repositories::comments::CommentRepo,
    error::AppError,
    services::context::RequestContext,
    validation::comment::{validate_create_comment, validate_update_comment},
};

pub struct CommentsService;

impl CommentsService {
    pub fn list_by_issue(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        issue_id: Uuid,
        include_deleted: bool,
    ) -> Result<Vec<Comment>, AppError> {
        CommentRepo::list_by_issue(conn, issue_id, include_deleted)
            .map_err(|e| AppError::internal(&format!("Failed to list comments: {}", e)))
    }

    pub fn create(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        issue_id: Uuid,
        content: String,
    ) -> Result<Comment, AppError> {
        validate_create_comment(&content)?;

        let _now = Utc::now().naive_utc();
        let new_comment = NewComment {
            issue_id,
            author_id: ctx.user_id,
            content,
            content_type: None,
            parent_comment_id: None,
        };

        CommentRepo::insert(conn, &new_comment)
            .map_err(|e| AppError::internal(&format!("Failed to create comment: {}", e)))
    }

    pub fn update(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        comment_id: Uuid,
        content: String,
    ) -> Result<Comment, AppError> {
        validate_update_comment(&content)?;

        // Check if comment exists and belongs to user
        let comment = CommentRepo::find_by_id(conn, comment_id)
            .map_err(|e| AppError::internal(&format!("Failed to find comment: {}", e)))?
            .ok_or_else(|| AppError::not_found("comment"))?;

        if comment.author_id != ctx.user_id {
            return Err(AppError::auth("You can only edit your own comments"));
        }

        CommentRepo::update_content(conn, comment_id, content)
            .map_err(|e| AppError::internal(&format!("Failed to update comment: {}", e)))
    }

    pub fn delete(
        conn: &mut PgConnection,
        ctx: &RequestContext,
        comment_id: Uuid,
    ) -> Result<(), AppError> {
        // Check if comment exists and belongs to user
        let comment = CommentRepo::find_by_id(conn, comment_id)
            .map_err(|e| AppError::internal(&format!("Failed to find comment: {}", e)))?
            .ok_or_else(|| AppError::not_found("comment"))?;

        if comment.author_id != ctx.user_id {
            return Err(AppError::auth("You can only delete your own comments"));
        }

        CommentRepo::soft_delete(conn, comment_id)
            .map_err(|e| AppError::internal(&format!("Failed to delete comment: {}", e)))?;

        Ok(())
    }

    pub fn get_by_id(
        conn: &mut PgConnection,
        _ctx: &RequestContext,
        comment_id: Uuid,
    ) -> Result<Comment, AppError> {
        CommentRepo::find_by_id(conn, comment_id)
            .map_err(|e| AppError::internal(&format!("Failed to find comment: {}", e)))?
            .ok_or_else(|| AppError::not_found("comment"))
    }
}
