use diesel::prelude::*;
use serde::Serialize;

#[derive(diesel_derive_enum::DbEnum, Serialize, Debug)]
#[ExistingTypePath = "crate::database::schema::sql_types::UserRole"]
#[DbValueStyle = "UPPERCASE"]
#[serde(rename_all = "UPPERCASE")]
pub enum UserRole {
    Student,
    Teacher,
    Admin,
}

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::database::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
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
