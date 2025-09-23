use diesel::prelude::*;

use crate::db::models::comment::{Comment, NewComment};

pub struct CommentRepo;

impl CommentRepo {
    pub fn find_by_id(
        conn: &mut PgConnection,
        comment_id: uuid::Uuid,
    ) -> Result<Option<Comment>, diesel::result::Error> {
        use crate::schema::comments::dsl::*;
        comments.filter(id.eq(comment_id)).first::<Comment>(conn).optional()
    }

    pub fn list_by_issue(
        conn: &mut PgConnection,
        target_issue_id: uuid::Uuid,
        include_deleted: bool,
    ) -> Result<Vec<Comment>, diesel::result::Error> {
        use crate::schema::comments::dsl::*;
        let mut query = comments.filter(issue_id.eq(target_issue_id)).into_boxed();

        if !include_deleted {
            query = query.filter(
                is_deleted.is_null().or(is_deleted.eq(false))
            );
        }

        query.order(created_at.desc()).load::<Comment>(conn)
    }

    pub fn insert(
        conn: &mut PgConnection,
        new_comment: &NewComment,
    ) -> Result<Comment, diesel::result::Error> {
        diesel::insert_into(crate::schema::comments::table)
            .values(new_comment)
            .get_result(conn)
    }

    pub fn update_content(
        conn: &mut PgConnection,
        comment_id: uuid::Uuid,
        new_content: String,
    ) -> Result<Comment, diesel::result::Error> {
        use crate::schema::comments::dsl::*;
        diesel::update(comments.filter(id.eq(comment_id)))
            .set(content.eq(new_content))
            .get_result(conn)
    }

    pub fn soft_delete(
        conn: &mut PgConnection,
        comment_id: uuid::Uuid,
    ) -> Result<Comment, diesel::result::Error> {
        use crate::schema::comments::dsl::*;
        diesel::update(comments.filter(id.eq(comment_id)))
            .set(is_deleted.eq(true))
            .get_result(conn)
    }

    pub fn hard_delete(
        conn: &mut PgConnection,
        comment_id: uuid::Uuid,
    ) -> Result<usize, diesel::result::Error> {
        use crate::schema::comments::dsl::*;
        diesel::delete(comments.filter(id.eq(comment_id))).execute(conn)
    }

    pub fn find_by_id_with_issue(
        conn: &mut PgConnection,
        comment_id: uuid::Uuid,
    ) -> Result<Option<(Comment, uuid::Uuid)>, diesel::result::Error> {
        use crate::schema::comments::dsl::*;
        comments
            .filter(id.eq(comment_id))
            .select((Comment::as_select(), issue_id))
            .first(conn)
            .optional()
    }
}
