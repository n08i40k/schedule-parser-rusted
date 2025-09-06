pub mod query;

pub use migration;
pub use sea_orm;

pub mod entity {
    pub use entity::*;

    pub use entity::user::{ActiveModel as ActiveUser, Model as User, Entity as UserEntity, Column as UserColumn};
}
