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
