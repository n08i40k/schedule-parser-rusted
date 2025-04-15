use actix_macros::ResponderJson;
use diesel::QueryId;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Copy, Clone, PartialEq, Debug, Serialize, Deserialize, diesel_derive_enum::DbEnum, ToSchema,
)]
#[ExistingTypePath = "crate::database::schema::sql_types::UserRole"]
#[DbValueStyle = "UPPERCASE"]
#[serde(rename_all = "UPPERCASE")]
pub enum UserRole {
    Student,
    Teacher,
    Admin,
}

#[derive(
    Identifiable,
    AsChangeset,
    Queryable,
    QueryId,
    Selectable,
    Serialize,
    Insertable,
    Debug,
    ToSchema,
    ResponderJson,
)]
#[diesel(table_name = crate::database::schema::users)]
#[diesel(treat_none_as_null = true)]
pub struct User {
    /// Account UUID.
    pub id: String,

    /// User name.
    pub username: String,

    /// BCrypt password hash.
    pub password: String,

    /// ID of the linked VK account.
    pub vk_id: Option<i32>,

    /// JWT access token.
    pub access_token: String,

    /// Group.
    pub group: String,

    /// Role.
    pub role: UserRole,

    /// Version of the installed Polytechnic+ application.
    pub version: String,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Identifiable,
    Queryable,
    Selectable,
    Insertable,
    AsChangeset,
    Associations,
    ToSchema,
    ResponderJson,
)]
#[diesel(belongs_to(User))]
#[diesel(table_name = crate::database::schema::fcm)]
#[diesel(primary_key(user_id))]
pub struct FCM {
    /// Account UUID.
    pub user_id: String,

    /// FCM token.
    pub token: String,

    /// List of topics subscribed to by the user.
    pub topics: Vec<Option<String>>,
}
