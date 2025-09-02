pub mod users {
    use crate::database::models::User;
    use crate::database::schema::users::dsl::users;
    use crate::database::schema::users::dsl::*;
    use crate::state::AppState;
    use actix_web::web;
    use diesel::{insert_into, ExpressionMethods, QueryResult};
    use diesel::{QueryDsl, RunQueryDsl};
    use diesel::{SaveChangesDsl, SelectableHelper};
    use std::ops::DerefMut;

    pub async fn get(state: &web::Data<AppState>, _id: &String) -> QueryResult<User> {
        users
            .filter(id.eq(_id))
            .select(User::as_select())
            .first(state.get_database().await.deref_mut())
    }

    pub async fn get_by_username(
        state: &web::Data<AppState>,
        _username: &String,
    ) -> QueryResult<User> {
        users
            .filter(username.eq(_username))
            .select(User::as_select())
            .first(state.get_database().await.deref_mut())
    }

    //noinspection RsTraitObligations
    pub async fn get_by_vk_id(state: &web::Data<AppState>, _vk_id: i32) -> QueryResult<User> {
        users
            .filter(vk_id.eq(_vk_id))
            .select(User::as_select())
            .first(state.get_database().await.deref_mut())
    }

    //noinspection RsTraitObligations
    pub async fn get_by_telegram_id(
        state: &web::Data<AppState>,
        _telegram_id: i64,
    ) -> QueryResult<User> {
        users
            .filter(telegram_id.eq(_telegram_id))
            .select(User::as_select())
            .first(state.get_database().await.deref_mut())
    }

    //noinspection DuplicatedCode
    pub async fn contains_by_username(state: &web::Data<AppState>, _username: &String) -> bool {
        // и как это нахуй сократить блять примеров нихуя нет, нихуя не работает
        // как меня этот раст заебал уже

        match users
            .filter(username.eq(_username))
            .count()
            .get_result::<i64>(state.get_database().await.deref_mut())
        {
            Ok(count) => count > 0,
            Err(_) => false,
        }
    }

    //noinspection DuplicatedCode
    //noinspection RsTraitObligations
    pub async fn contains_by_vk_id(state: &web::Data<AppState>, _vk_id: i32) -> bool {
        match users
            .filter(vk_id.eq(_vk_id))
            .count()
            .get_result::<i64>(state.get_database().await.deref_mut())
        {
            Ok(count) => count > 0,
            Err(_) => false,
        }
    }

    pub async fn insert(state: &web::Data<AppState>, user: &User) -> QueryResult<usize> {
        insert_into(users)
            .values(user)
            .execute(state.get_database().await.deref_mut())
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
        async fn save(&self, state: &web::Data<AppState>) -> QueryResult<User>;
    }

    /// Implementation of [UserSave][UserSave] trait.
    impl UserSave for User {
        async fn save(&self, state: &web::Data<AppState>) -> QueryResult<User> {
            self.save_changes::<Self>(state.get_database().await.deref_mut())
        }
    }

    #[cfg(test)]
    pub async fn delete_by_username(state: &web::Data<AppState>, _username: &String) -> bool {
        match diesel::delete(users.filter(username.eq(_username)))
            .execute(state.get_database().await.deref_mut())
        {
            Ok(count) => count > 0,
            Err(_) => false,
        }
    }

    #[cfg(test)]
    pub async fn insert_or_ignore(state: &web::Data<AppState>, user: &User) -> QueryResult<usize> {
        insert_into(users)
            .values(user)
            .on_conflict_do_nothing()
            .execute(state.get_database().await.deref_mut())
    }
}
