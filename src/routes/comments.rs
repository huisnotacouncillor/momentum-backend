use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;
use crate::db::models::auth::User;
use crate::db::models::comment::*;
use crate::middleware::auth::AuthUserInfo;

#[derive(Deserialize)]
pub struct CommentQueryParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub include_deleted: Option<bool>,
}

#[derive(Serialize)]
pub struct CommentsResponse {
    pub comments: Vec<CommentWithDetails>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
}

// 获取issue的评论列表
pub async fn get_comments(
    State(state): State<Arc<AppState>>,
    Path(issue_id): Path<Uuid>,
    Query(params): Query<CommentQueryParams>,
    _auth_info: AuthUserInfo,
) -> Result<Json<CommentsResponse>, StatusCode> {
    use crate::schema::comments;

    let mut conn = state
        .db
        .get()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    let offset = (page - 1) * limit;
    let include_deleted = params.include_deleted.unwrap_or(false);

    // 构建查询
    let mut query = comments::table
        .filter(comments::issue_id.eq(issue_id))
        .into_boxed();

    if !include_deleted {
        query = query.filter(
            comments::is_deleted
                .is_null()
                .or(comments::is_deleted.eq(false)),
        );
    }

    // 获取顶级评论（非回复）
    let top_level_comments: Vec<Comment> = query
        .filter(comments::parent_comment_id.is_null())
        .order(comments::created_at.asc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 获取总数
    let total: i64 = comments::table
        .filter(comments::issue_id.eq(issue_id))
        .filter(comments::parent_comment_id.is_null())
        .count()
        .get_result(&mut conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 为每个评论获取详细信息
    let mut comments_with_details = Vec::new();

    for comment in top_level_comments {
        let comment_with_details = build_comment_with_details(&mut conn, comment)?;
        comments_with_details.push(comment_with_details);
    }

    Ok(Json(CommentsResponse {
        comments: comments_with_details,
        total,
        page,
        limit,
    }))
}

// 创建新评论
pub async fn create_comment(
    State(state): State<Arc<AppState>>,
    Path(issue_id): Path<Uuid>,
    auth_info: AuthUserInfo,
    Json(payload): Json<CreateCommentRequest>,
) -> Result<Json<CommentWithDetails>, StatusCode> {
    use crate::schema::{comment_attachments, comment_mentions, comments};

    let mut conn = state
        .db
        .get()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 验证父评论是否存在（如果指定了）
    if let Some(parent_id) = payload.parent_comment_id {
        let parent_count: i64 = comments::table
            .filter(comments::id.eq(parent_id))
            .filter(comments::issue_id.eq(issue_id))
            .count()
            .get_result(&mut conn)
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        if parent_count == 0 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // 创建评论
    let new_comment = NewComment {
        issue_id,
        author_id: auth_info.user.id,
        content: payload.content,
        content_type: payload.content_type.or(Some("markdown".to_string())),
        parent_comment_id: payload.parent_comment_id,
    };

    let comment: Comment = diesel::insert_into(comments::table)
        .values(&new_comment)
        .get_result(&mut conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 处理@提及
    if let Some(mentions) = payload.mentions {
        let comment_mentions: Vec<NewCommentMention> = mentions
            .into_iter()
            .map(|user_id| NewCommentMention {
                comment_id: comment.id,
                mentioned_user_id: user_id,
            })
            .collect();

        if !comment_mentions.is_empty() {
            diesel::insert_into(comment_mentions::table)
                .values(&comment_mentions)
                .execute(&mut conn)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    // 处理附件
    if let Some(attachments) = payload.attachments {
        let comment_attachments: Vec<NewCommentAttachment> = attachments
            .into_iter()
            .map(|mut attachment| {
                attachment.comment_id = comment.id;
                attachment
            })
            .collect();

        if !comment_attachments.is_empty() {
            diesel::insert_into(comment_attachments::table)
                .values(&comment_attachments)
                .execute(&mut conn)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    // 返回完整的评论信息
    let comment_with_details = build_comment_with_details(&mut conn, comment)?;

    Ok(Json(comment_with_details))
}

// 获取单个评论
pub async fn get_comment_by_id(
    State(state): State<Arc<AppState>>,
    Path(comment_id): Path<Uuid>,
    _auth_info: AuthUserInfo,
) -> Result<Json<CommentWithDetails>, StatusCode> {
    let mut conn = state
        .db
        .get()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let comment: Comment = crate::schema::comments::table
        .find(comment_id)
        .first(&mut conn)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let comment_with_details = build_comment_with_details(&mut conn, comment)?;

    Ok(Json(comment_with_details))
}

// 更新评论
pub async fn update_comment(
    State(state): State<Arc<AppState>>,
    Path(comment_id): Path<Uuid>,
    auth_info: AuthUserInfo,
    Json(payload): Json<UpdateCommentRequest>,
) -> Result<Json<CommentWithDetails>, StatusCode> {
    use crate::schema::comments;

    let mut conn = state
        .db
        .get()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 验证评论所有权
    let comment: Comment = comments::table
        .find(comment_id)
        .first(&mut conn)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    if comment.author_id != auth_info.user.id {
        return Err(StatusCode::FORBIDDEN);
    }

    // 更新评论
    let update_data = UpdateComment {
        content: payload.content,
        content_type: payload.content_type,
        is_edited: Some(true),
        updated_at: chrono::Utc::now(),
    };

    let updated_comment: Comment = diesel::update(comments::table.find(comment_id))
        .set(&update_data)
        .get_result(&mut conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let comment_with_details = build_comment_with_details(&mut conn, updated_comment)?;

    Ok(Json(comment_with_details))
}

// 删除评论（软删除）
pub async fn delete_comment(
    State(state): State<Arc<AppState>>,
    Path(comment_id): Path<Uuid>,
    auth_info: AuthUserInfo,
) -> Result<StatusCode, StatusCode> {
    use crate::schema::comments;

    let mut conn = state
        .db
        .get()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 验证评论所有权
    let comment: Comment = comments::table
        .find(comment_id)
        .first(&mut conn)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    if comment.author_id != auth_info.user.id {
        return Err(StatusCode::FORBIDDEN);
    }

    // 软删除评论
    diesel::update(comments::table.find(comment_id))
        .set((
            comments::is_deleted.eq(true),
            comments::updated_at.eq(chrono::Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

// 添加表情反应
pub async fn add_reaction(
    State(state): State<Arc<AppState>>,
    Path(comment_id): Path<Uuid>,
    auth_info: AuthUserInfo,
    Json(payload): Json<NewCommentReaction>,
) -> Result<Json<CommentReaction>, StatusCode> {
    use crate::schema::comment_reactions;

    let mut conn = state
        .db
        .get()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let new_reaction = NewCommentReaction {
        comment_id,
        user_id: auth_info.user.id,
        reaction_type: payload.reaction_type,
    };

    let reaction: CommentReaction = diesel::insert_into(comment_reactions::table)
        .values(&new_reaction)
        .on_conflict((
            comment_reactions::comment_id,
            comment_reactions::user_id,
            comment_reactions::reaction_type,
        ))
        .do_nothing()
        .get_result(&mut conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(reaction))
}

// 移除表情反应
pub async fn remove_reaction(
    State(state): State<Arc<AppState>>,
    Path((comment_id, reaction_type)): Path<(Uuid, String)>,
    auth_info: AuthUserInfo,
) -> Result<StatusCode, StatusCode> {
    use crate::schema::comment_reactions;

    let mut conn = state
        .db
        .get()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    diesel::delete(
        comment_reactions::table
            .filter(comment_reactions::comment_id.eq(comment_id))
            .filter(comment_reactions::user_id.eq(auth_info.user.id))
            .filter(comment_reactions::reaction_type.eq(reaction_type)),
    )
    .execute(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

// 辅助函数：构建包含详细信息的评论
fn build_comment_with_details(
    conn: &mut PgConnection,
    comment: Comment,
) -> Result<CommentWithDetails, StatusCode> {
    use crate::schema::{
        comment_attachments, comment_mentions, comment_reactions, comments, users,
    };

    // 获取作者信息
    let author: Option<User> = users::table
        .find(comment.author_id)
        .first(conn)
        .optional()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 获取提及信息
    let mentions: Vec<CommentMention> = comment_mentions::table
        .filter(comment_mentions::comment_id.eq(comment.id))
        .load(conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 获取附件信息
    let attachments: Vec<CommentAttachment> = comment_attachments::table
        .filter(comment_attachments::comment_id.eq(comment.id))
        .load(conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 获取反应信息
    let reactions: Vec<CommentReaction> = comment_reactions::table
        .filter(comment_reactions::comment_id.eq(comment.id))
        .load(conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 获取回复（递归）
    let reply_comments: Vec<Comment> = comments::table
        .filter(comments::parent_comment_id.eq(comment.id))
        .filter(
            comments::is_deleted
                .is_null()
                .or(comments::is_deleted.eq(false)),
        )
        .order(comments::created_at.asc())
        .load(conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut replies = Vec::new();
    for reply in reply_comments {
        let reply_with_details = build_comment_with_details(conn, reply)?;
        replies.push(reply_with_details);
    }

    Ok(CommentWithDetails {
        comment,
        author,
        mentions,
        attachments,
        reactions,
        replies,
    })
}
