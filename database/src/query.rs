use paste::paste;
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;

pub struct Query;

macro_rules! ref_type {
    (String) => {
        &String
    };
    (str) => {
        &str
    };
    ($other:ty) => {
        $other
    };
}

macro_rules! define_is_exists {
    ($entity: ident, $by: ident, $by_type: ident, $by_column: ident) => {
        paste! {
            pub async fn [<is_ $entity _exists_by_ $by>](
                db: &::sea_orm::DbConn,
                $by: ref_type!($by_type)
            ) -> Result<bool, ::sea_orm::DbErr> {
                ::entity::$entity::Entity::find()
                    .filter(::entity::$entity::Column::$by_column.eq($by))
                    .one(db)
                    .await
                    .map(|x| x.is_some())
            }
        }
    };
}

macro_rules! define_find_by {
    ($entity: ident, $by: ident, $by_type: ident, $by_column: ident) => {
        paste! {
            pub async fn [<find_ $entity _by_ $by>](
                db: &::sea_orm::DbConn,
                $by: ref_type!($by_type)
            ) -> Result<Option<::entity::$entity::Model>, ::sea_orm::DbErr> {
                ::entity::$entity::Entity::find()
                    .filter(::entity::$entity::Column::$by_column.eq($by))
                    .one(db)
                    .await
            }
        }
    };
}

impl Query {
    define_find_by!(user, id, str, Id);
    define_find_by!(user, telegram_id, i64, TelegramId);
    define_find_by!(user, vk_id, i32, VkId);
    define_find_by!(user, username, str, Username);

    define_is_exists!(user, id, str, Id);
    define_is_exists!(user, username, str, Username);
    define_is_exists!(user, telegram_id, i64, TelegramId);
    define_is_exists!(user, vk_id, i32, VkId);
}
