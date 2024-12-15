use sea_orm::sea_query::{
    ColumnDef, Expr, IntoIden, Table, TableAlterStatement, TableCreateStatement,
};
pub use sea_orm_migration::schema::*;
use sea_orm_migration::{prelude::Iden, sea_query};

#[derive(Iden)]
enum GeneralIds {
    CreatedAt,
    UpdatedAt,
}

/// Alter table
pub fn alter<T: IntoIden + 'static>(name: T) -> TableAlterStatement {
    Table::alter().table(name).take()
}

/// Wrapping  table schema creation.
pub fn table_auto_tz<T>(name: T) -> TableCreateStatement
where
    T: IntoIden + 'static,
{
    timestamps_tz(Table::create().table(name).if_not_exists().take())
}

// these two are just aliases, original types exist in seaorm already.

#[must_use]
pub fn timestamps_tz(t: TableCreateStatement) -> TableCreateStatement {
    let mut t = t;
    t.col(timestamp_with_time_zone(GeneralIds::CreatedAt).default(Expr::current_timestamp()))
        .col(timestamp_with_time_zone(GeneralIds::UpdatedAt).default(Expr::current_timestamp()));
    t.take()
}

/// Create a nullable timestamptz column definition.
pub fn timestamptz_null<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).timestamp_with_time_zone().take()
}

/// Create a non-nullable timestamptz column definition.
pub fn timestamptz<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name)
        .timestamp_with_time_zone()
        .not_null()
        .take()
}
