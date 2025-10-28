pub mod query;

pub use migration;
pub use sea_orm;

pub mod entity {
    use serde::{Deserialize, Serialize};

    pub use entity::*;

    pub use entity::user::{
        ActiveModel as ActiveUser, //
        Column as UserColumn,      //
        Entity as UserEntity,      //
        Model as User,             //
    };

    pub use entity::service_user::{
        ActiveModel as ActiveServiceUser, //
        Column as ServiceUserColumn,      //
        Entity as ServiceUserEntity,      //
        Model as ServiceUser,             //
    };

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum UserType {
        Default,
        Service,
    }
}
