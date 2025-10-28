pub use sea_orm_migration::prelude::MigratorTrait;

use sea_orm_migration::prelude::*;

mod m20250904_024854_init;
mod m20251027_230335_add_service_users;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250904_024854_init::Migration),
            Box::new(m20251027_230335_add_service_users::Migration),
        ]
    }
}
