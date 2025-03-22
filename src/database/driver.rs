pub mod users {
    use crate::database::models::User;
    use crate::database::schema::fcm::user_id;
    use crate::database::schema::users::dsl::users;
    use crate::database::schema::users::dsl::*;
    use diesel::{ExpressionMethods, QueryResult};
    use diesel::{PgConnection, SelectableHelper};
    use diesel::{QueryDsl, RunQueryDsl};
    use std::ops::DerefMut;
    use std::sync::Mutex;

    pub fn get(connection: &Mutex<PgConnection>, _id: String) -> QueryResult<User> {
        let mut lock = connection.lock().unwrap();
        let con = lock.deref_mut();

        users
            .filter(id.eq(_id))
            .select(User::as_select())
            .first(con)
    }

    pub fn get_by_username(
        connection: &Mutex<PgConnection>,
        _username: String,
    ) -> QueryResult<User> {
        let mut lock = connection.lock().unwrap();
        let con = lock.deref_mut();

        users
            .filter(username.eq(_username))
            .select(User::as_select())
            .first(con)
    }
}
