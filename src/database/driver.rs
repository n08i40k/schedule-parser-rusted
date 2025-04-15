pub mod users {
    use crate::app_state::AppState;
    use crate::database::models::User;
    use crate::database::schema::users::dsl::users;
    use crate::database::schema::users::dsl::*;
    use crate::utility::mutex::MutexScope;
    use actix_web::web;
    use diesel::{ExpressionMethods, QueryResult, insert_into};
    use diesel::{QueryDsl, RunQueryDsl};
    use diesel::{SaveChangesDsl, SelectableHelper};

    pub fn get(state: &web::Data<AppState>, _id: &String) -> QueryResult<User> {
        state.database.scope(|conn| {
            users
                .filter(id.eq(_id))
                .select(User::as_select())
                .first(conn)
        })
    }

    pub fn get_by_username(state: &web::Data<AppState>, _username: &String) -> QueryResult<User> {
        state.database.scope(|conn| {
            users
                .filter(username.eq(_username))
                .select(User::as_select())
                .first(conn)
        })
    }

    //noinspection RsTraitObligations
    pub fn get_by_vk_id(state: &web::Data<AppState>, _vk_id: i32) -> QueryResult<User> {
        state.database.scope(|conn| {
            users
                .filter(vk_id.eq(_vk_id))
                .select(User::as_select())
                .first(conn)
        })
    }

    //noinspection DuplicatedCode
    pub fn contains_by_username(state: &web::Data<AppState>, _username: &String) -> bool {
        // и как это нахуй сократить блять примеров нихуя нет, нихуя не работает
        // как меня этот раст заебал уже
        state.database.scope(|conn| {
            match users
                .filter(username.eq(_username))
                .count()
                .get_result::<i64>(conn)
            {
                Ok(count) => count > 0,
                Err(_) => false,
            }
        })
    }

    //noinspection DuplicatedCode
    //noinspection RsTraitObligations
    pub fn contains_by_vk_id(state: &web::Data<AppState>, _vk_id: i32) -> bool {
        state.database.scope(|conn| {
            match users
                .filter(vk_id.eq(_vk_id))
                .count()
                .get_result::<i64>(conn)
            {
                Ok(count) => count > 0,
                Err(_) => false,
            }
        })
    }

    pub fn insert(state: &web::Data<AppState>, user: &User) -> QueryResult<usize> {
        state
            .database
            .scope(|conn| insert_into(users).values(user).execute(conn))
    }

    /// Function declaration [User::save][UserSave::save].
    pub trait UserSave {
        /// Saves the user's changes to the database.
        ///
        /// # Arguments
        ///
        /// * `state`: The state of the actix-web application that stores the mutex of the [connection][diesel::PgConnection].
        ///
        /// returns: `QueryResult<User>`
        ///
        /// # Examples
        ///
        /// ```
        /// use crate::database::driver::users;
        ///
        /// #[derive(Deserialize)]
        /// struct Params {
        ///     pub username: String,
        /// }
        ///
        /// #[patch("/")]
        /// async fn patch_user(
        ///     app_state: web::Data<AppState>,
        ///     user: SyncExtractor<User>,
        ///     web::Query(params): web::Query<Params>,
        /// ) -> web::Json<User> {
        ///     let mut user = user.into_inner();
        ///
        ///     user.username = params.username;
        ///
        ///     match user.save(&app_state) {
        ///         Ok(user) => web::Json(user),
        ///         Err(e) => {
        ///             eprintln!("Failed to save user: {e}");
        ///             panic!();
        ///         }
        ///     }
        /// }
        /// ```
        fn save(&self, state: &web::Data<AppState>) -> QueryResult<User>;
    }

    /// Implementation of [UserSave][UserSave] trait.
    impl UserSave for User {
        fn save(&self, state: &web::Data<AppState>) -> QueryResult<User> {
            state.database.scope(|conn| self.save_changes::<Self>(conn))
        }
    }

    #[cfg(test)]
    pub fn delete_by_username(state: &web::Data<AppState>, _username: &String) -> bool {
        state.database.scope(|conn| {
            match diesel::delete(users.filter(username.eq(_username))).execute(conn) {
                Ok(count) => count > 0,
                Err(_) => false,
            }
        })
    }

    #[cfg(test)]
    pub fn insert_or_ignore(state: &web::Data<AppState>, user: &User) -> QueryResult<usize> {
        state.database.scope(|conn| {
            insert_into(users)
                .values(user)
                .on_conflict_do_nothing()
                .execute(conn)
        })
    }
}

pub mod fcm {
    use crate::app_state::AppState;
    use crate::database::models::{FCM, User};
    use crate::utility::mutex::MutexScope;
    use actix_web::web;
    use diesel::QueryDsl;
    use diesel::RunQueryDsl;
    use diesel::{BelongingToDsl, QueryResult, SelectableHelper};

    pub fn from_user(state: &web::Data<AppState>, user: &User) -> QueryResult<FCM> {
        state.database.scope(|conn| {
            FCM::belonging_to(&user)
                .select(FCM::as_select())
                .get_result(conn)
        })
    }
}
