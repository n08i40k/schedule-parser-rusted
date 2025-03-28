pub mod users {
    use crate::database::models::User;
    use crate::database::schema::users::dsl::users;
    use crate::database::schema::users::dsl::*;
    use diesel::{ExpressionMethods, QueryResult, insert_into};
    use diesel::{PgConnection, SelectableHelper};
    use diesel::{QueryDsl, RunQueryDsl};
    use std::ops::DerefMut;
    use std::sync::Mutex;

    pub fn get(connection: &Mutex<PgConnection>, _id: &String) -> QueryResult<User> {
        let mut lock = connection.lock().unwrap();
        let con = lock.deref_mut();

        users
            .filter(id.eq(_id))
            .select(User::as_select())
            .first(con)
    }

    pub fn get_by_username(
        connection: &Mutex<PgConnection>,
        _username: &String,
    ) -> QueryResult<User> {
        let mut lock = connection.lock().unwrap();
        let con = lock.deref_mut();

        users
            .filter(username.eq(_username))
            .select(User::as_select())
            .first(con)
    }

    pub fn get_by_vk_id(
        connection: &Mutex<PgConnection>,
        _vk_id: i32,
    ) -> QueryResult<User> {
        let mut lock = connection.lock().unwrap();
        let con = lock.deref_mut();

        users
            .filter(vk_id.eq(_vk_id))
            .select(User::as_select())
            .first(con)
    }

    pub fn contains_by_username(connection: &Mutex<PgConnection>, _username: &String) -> bool {
        let mut lock = connection.lock().unwrap();
        let con = lock.deref_mut();

        match users
            .filter(username.eq(_username))
            .count()
            .get_result::<i64>(con)
        {
            Ok(count) => count > 0,
            Err(_) => false,
        }
    }

    pub fn contains_by_vk_id(connection: &Mutex<PgConnection>, _vk_id: i32) -> bool {
        let mut lock = connection.lock().unwrap();
        let con = lock.deref_mut();

        match users
            .filter(vk_id.eq(_vk_id))
            .count()
            .get_result::<i64>(con)
        {
            Ok(count) => count > 0,
            Err(_) => false,
        }
    }

    pub fn insert(connection: &Mutex<PgConnection>, user: &User) -> QueryResult<usize> {
        let mut lock = connection.lock().unwrap();
        let con = lock.deref_mut();

        insert_into(users).values(user).execute(con)
    }

    #[cfg(test)]
    pub fn delete_by_username(connection: &Mutex<PgConnection>, _username: &String) -> bool {
        let mut lock = connection.lock().unwrap();
        let con = lock.deref_mut();

        match diesel::delete(users.filter(username.eq(_username))).execute(con) {
            Ok(count) => count > 0,
            Err(_) => false,
        }
    }
    
    #[cfg(test)]
    pub fn insert_or_ignore(connection: &Mutex<PgConnection>, user: &User) -> QueryResult<usize> {
        let mut lock = connection.lock().unwrap();
        let con = lock.deref_mut();

        insert_into(users)
            .values(user)
            .on_conflict_do_nothing()
            .execute(con)
    }
}
