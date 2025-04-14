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
    /// UUID аккаунта
    pub id: String,

    /// Имя пользователя
    pub username: String,

    /// BCrypt хеш пароля
    pub password: String,

    /// Идентификатор привязанного аккаунта VK
    pub vk_id: Option<i32>,

    /// JWT токен доступа
    pub access_token: String,

    /// Группа
    pub group: String,

    /// Роль
    pub role: UserRole,

    /// Версия установленного приложения Polytechnic+
    pub version: String,
}

#[derive(
    Debug,
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
    /// UUID аккаунта.
    pub user_id: String,

    /// FCM токен.
    pub token: String,

    /// Список топиков, на которые подписан пользователь.
    pub topics: Vec<Option<String>>,
}
