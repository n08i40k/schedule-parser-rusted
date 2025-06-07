// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "user_role"))]
    pub struct UserRole;
}

diesel::table! {
    fcm (user_id) {
        user_id -> Text,
        token -> Text,
        topics -> Array<Nullable<Text>>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::UserRole;

    users (id) {
        id -> Text,
        username -> Text,
        password -> Nullable<Text>,
        vk_id -> Nullable<Int4>,
        access_token -> Nullable<Text>,
        group -> Nullable<Text>,
        role -> UserRole,
        android_version -> Nullable<Text>,
        telegram_id -> Nullable<Int8>,
    }
}

diesel::joinable!(fcm -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    fcm,
    users,
);
