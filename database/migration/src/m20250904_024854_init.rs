use sea_orm_migration::prelude::extension::postgres::Type;
use sea_orm_migration::sea_orm::{EnumIter, Iterable};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(UserRole)
                    .values(UserRoleVariants::iter())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(string_uniq(User::Id).primary_key().not_null())
                    .col(string_uniq(User::Username).not_null())
                    .col(string_null(User::Password))
                    .col(integer_null(User::VkId))
                    .col(string_null(User::Group))
                    .col(enumeration(User::Role, UserRole, UserRoleVariants::iter()))
                    .col(string_null(User::AndroidVersion))
                    .col(big_integer_null(User::TelegramId).unique_key())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await?;

        manager
            .drop_type(Type::drop().name(UserRole).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
struct UserRole;

#[derive(DeriveIden, EnumIter)]
enum UserRoleVariants {
    Student,
    Teacher,
    Admin,
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Username,
    Password,
    VkId,
    Group,
    Role,
    AndroidVersion,
    TelegramId,
}
