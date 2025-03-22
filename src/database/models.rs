use diesel::prelude::*;
use serde::Serialize;

#[derive(diesel_derive_enum::DbEnum, Serialize, Debug, Clone, Copy, PartialEq)]
#[ExistingTypePath = "crate::database::schema::sql_types::UserRole"]
#[DbValueStyle = "UPPERCASE"]
#[serde(rename_all = "UPPERCASE")]
pub enum UserRole {
    Student,
    Teacher,
    Admin,
}

#[derive(Identifiable, AsChangeset, Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::database::schema::users)]
#[diesel(treat_none_as_null = true)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password: String,
    pub vk_id: Option<i32>,
    pub access_token: String,
    pub group: String,
    pub role: UserRole,
    pub version: String,
}
