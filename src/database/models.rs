use actix_macros::ResponderJson;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    diesel_derive_enum::DbEnum,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    utoipa::ToSchema,
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
    Selectable,
    Serialize,
    Insertable,
    Debug,
    utoipa::ToSchema,
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
