#[cfg(feature = "channels")]
use crate::controller::channels::AppChannels;
use crate::{
    app::{AppContext, Hooks, Initializer},
    boot::{create_app, BootResult, StartMode},
    controller::AppRoutes,
    environment::Environment,
    task::Tasks,
    worker::Processor,
    Result,
};

use std::path::Path;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;
pub use sea_orm_migration::prelude::*;

/// Creating a dummy db connection for docs
///
/// # Panics
/// Disabled the connection validation, should pass always
pub async fn dummy_connection() -> sea_orm::DatabaseConnection {
    let mut opt = sea_orm::ConnectOptions::new("sqlite::memory:");
    opt.test_before_acquire(false);

    sea_orm::Database::connect(opt).await.unwrap()
}

pub mod test_db {
    use std::fmt;

    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "loco")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
        pub created_at: DateTime,
        pub updated_at: DateTime,
    }

    #[derive(Debug)]
    pub enum Loco {
        Table,
        Id,
        Name,
    }

    impl Iden for Loco {
        fn unquoted(&self, s: &mut dyn fmt::Write) {
            write!(
                s,
                "{}",
                match self {
                    Self::Table => "loco",
                    Self::Id => "id",
                    Self::Name => "name",
                }
            )
            .unwrap();
        }
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![]
    }
}

pub struct AppHook;
#[async_trait]
impl Hooks for AppHook {
    fn app_version() -> String {
        "test".to_string()
    }

    fn app_name() -> &'static str {
        "TEST"
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
    }

    async fn boot(mode: StartMode, environment: &Environment) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment).await
    }

    fn connect_workers<'a>(_p: &'a mut Processor, _ctx: &'a AppContext) {}

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(super::task::Foo);
        tasks.register(super::task::ParseArgs);
    }

    async fn truncate(_db: &DatabaseConnection) -> Result<()> {
        Ok(())
    }

    async fn seed(_db: &DatabaseConnection, _base: &Path) -> Result<()> {
        Ok(())
    }

    #[cfg(feature = "channels")]
    #[allow(clippy::unimplemented)]
    fn register_channels(_ctx: &AppContext) -> AppChannels {
        unimplemented!();
    }
}
