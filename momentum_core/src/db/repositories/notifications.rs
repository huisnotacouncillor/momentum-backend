use diesel::prelude::*;

use crate::db::models::notification::{NewNotification, Notification};
use crate::schema::notifications;

pub struct NotificationsRepo;

impl NotificationsRepo {
    pub fn create(
        conn: &mut PgConnection,
        notification: &NewNotification,
    ) -> Result<Notification, diesel::result::Error> {
        diesel::insert_into(notifications::table)
            .values(notification)
            .get_result(conn)
    }

    pub fn list_by_user(
        conn: &mut PgConnection,
        user_id: uuid::Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Notification>, diesel::result::Error> {
        notifications::table
            .filter(notifications::user_id.eq(user_id))
            .order(notifications::created_at.desc())
            .limit(limit)
            .offset(offset)
            .load(conn)
    }

    pub fn mark_as_read(
        conn: &mut PgConnection,
        notification_id: uuid::Uuid,
        user_id: uuid::Uuid,
    ) -> Result<Notification, diesel::result::Error> {
        diesel::update(
            notifications::table
                .filter(notifications::id.eq(notification_id))
                .filter(notifications::user_id.eq(user_id)),
        )
        .set((
            notifications::is_read.eq(true),
            notifications::updated_at.eq(chrono::Utc::now()),
        ))
        .get_result(conn)
    }

    pub fn mark_all_as_read(
        conn: &mut PgConnection,
        user_id: uuid::Uuid,
    ) -> Result<usize, diesel::result::Error> {
        diesel::update(
            notifications::table
                .filter(notifications::user_id.eq(user_id))
                .filter(notifications::is_read.eq(false)),
        )
        .set((
            notifications::is_read.eq(true),
            notifications::updated_at.eq(chrono::Utc::now()),
        ))
        .execute(conn)
    }

    pub fn unread_count(
        conn: &mut PgConnection,
        user_id: uuid::Uuid,
    ) -> Result<i64, diesel::result::Error> {
        notifications::table
            .filter(notifications::user_id.eq(user_id))
            .filter(notifications::is_read.eq(false))
            .count()
            .get_result(conn)
    }
}