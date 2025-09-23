use diesel::prelude::*;

use crate::db::models::auth::{User, NewUser, UserCredential, NewUserCredential};

pub struct AuthRepo;

impl AuthRepo {
    pub fn find_by_email(
        conn: &mut PgConnection,
        target_email: &str,
    ) -> Result<Option<User>, diesel::result::Error> {
        use crate::schema::users::dsl::*;
        users.filter(email.eq(target_email)).first::<User>(conn).optional()
    }

    pub fn find_by_username(
        conn: &mut PgConnection,
        target_username: &str,
    ) -> Result<Option<User>, diesel::result::Error> {
        use crate::schema::users::dsl::*;
        users.filter(username.eq(target_username)).first::<User>(conn).optional()
    }

    pub fn find_by_id(
        conn: &mut PgConnection,
        user_id: uuid::Uuid,
    ) -> Result<Option<User>, diesel::result::Error> {
        use crate::schema::users::dsl::*;
        users.filter(id.eq(user_id)).first::<User>(conn).optional()
    }

    pub fn exists_by_email(
        conn: &mut PgConnection,
        target_email: &str,
    ) -> Result<bool, diesel::result::Error> {
        use crate::schema::users::dsl::*;
        diesel::select(diesel::dsl::exists(
            users.filter(email.eq(target_email))
        ))
        .get_result(conn)
    }

    pub fn exists_by_username(
        conn: &mut PgConnection,
        target_username: &str,
    ) -> Result<bool, diesel::result::Error> {
        use crate::schema::users::dsl::*;
        diesel::select(diesel::dsl::exists(
            users.filter(username.eq(target_username))
        ))
        .get_result(conn)
    }

    pub fn insert_user(
        conn: &mut PgConnection,
        new_user: &NewUser,
    ) -> Result<User, diesel::result::Error> {
        diesel::insert_into(crate::schema::users::table)
            .values(new_user)
            .get_result(conn)
    }

    pub fn insert_credential(
        conn: &mut PgConnection,
        new_credential: &NewUserCredential,
    ) -> Result<UserCredential, diesel::result::Error> {
        diesel::insert_into(crate::schema::user_credentials::table)
            .values(new_credential)
            .get_result(conn)
    }

    pub fn find_credential_by_user_id(
        conn: &mut PgConnection,
        target_user_id: uuid::Uuid,
    ) -> Result<Option<UserCredential>, diesel::result::Error> {
        use crate::schema::user_credentials::dsl::*;
        user_credentials.filter(user_id.eq(target_user_id)).first::<UserCredential>(conn).optional()
    }

    pub fn update_user_fields(
        conn: &mut PgConnection,
        user_id: uuid::Uuid,
        changes: (Option<String>, Option<String>, Option<String>, Option<String>),
    ) -> Result<User, diesel::result::Error> {
        use crate::schema::users::dsl as u;

        // Apply sets based on what fields are provided
        if let (Some(name_val), Some(username_val), Some(email_val), Some(avatar_url_val)) = (
            changes.0.clone(),
            changes.1.clone(),
            changes.2.clone(),
            changes.3.clone(),
        ) {
            return diesel::update(u::users.filter(u::id.eq(user_id)))
                .set((u::name.eq(name_val), u::username.eq(username_val), u::email.eq(email_val), u::avatar_url.eq(avatar_url_val)))
                .get_result(conn);
        }

        if let (Some(name_val), Some(username_val), Some(email_val)) = (
            changes.0.clone(),
            changes.1.clone(),
            changes.2.clone(),
        ) {
            return diesel::update(u::users.filter(u::id.eq(user_id)))
                .set((u::name.eq(name_val), u::username.eq(username_val), u::email.eq(email_val)))
                .get_result(conn);
        }

        if let (Some(name_val), Some(username_val)) = (changes.0.clone(), changes.1.clone()) {
            return diesel::update(u::users.filter(u::id.eq(user_id)))
                .set((u::name.eq(name_val), u::username.eq(username_val)))
                .get_result(conn);
        }

        if let Some(name_val) = changes.0.clone() {
            return diesel::update(u::users.filter(u::id.eq(user_id)))
                .set(u::name.eq(name_val))
                .get_result(conn);
        }

        if let Some(username_val) = changes.1.clone() {
            return diesel::update(u::users.filter(u::id.eq(user_id)))
                .set(u::username.eq(username_val))
                .get_result(conn);
        }

        if let Some(email_val) = changes.2.clone() {
            return diesel::update(u::users.filter(u::id.eq(user_id)))
                .set(u::email.eq(email_val))
                .get_result(conn);
        }

        if let Some(avatar_url_val) = changes.3.clone() {
            return diesel::update(u::users.filter(u::id.eq(user_id)))
                .set(u::avatar_url.eq(avatar_url_val))
                .get_result(conn);
        }

        // No changes provided, return current row
        use crate::schema::users::dsl::*;
        users.filter(id.eq(user_id)).first::<User>(conn)
    }

    pub fn update_current_workspace(
        conn: &mut PgConnection,
        target_user_id: uuid::Uuid,
        target_workspace_id: uuid::Uuid,
    ) -> Result<User, diesel::result::Error> {
        use crate::schema::users::dsl::*;
        diesel::update(users.filter(id.eq(target_user_id)))
            .set(current_workspace_id.eq(target_workspace_id))
            .get_result(conn)
    }
}
