/// Creating a dummy db connection for docs
///
/// # Panics
/// Disabled the connection validation, should pass always
pub async fn dummy_connection() -> sea_orm::DatabaseConnection {
    let mut opt = sea_orm::ConnectOptions::new("postgres://@dummy:5432/dummy");
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
