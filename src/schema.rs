//! # Database Table Schema Helpers
//!
//! This module defines functions and helpers for creating database table
//! schemas using the `sea-orm` and `sea-query` libraries.
//!
//! # Example
//!
//! The following example shows how the user migration file should be and using
//! the schema helpers to create the Db fields.
//!
//! ```rust
//! use loco_rs::schema::*;
//! use sea_orm_migration::prelude::*;
//! use std::borrow::BorrowMut;
//!
//! #[derive(DeriveMigrationName)]
//! pub struct Migration;
//!
//! #[async_trait::async_trait]
//! impl MigrationTrait for Migration {
//!     async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
//!         let table = table_auto(Users::Table)
//!             .col(pk_auto(Users::Id).borrow_mut())
//!             .col(uuid(Users::Pid).borrow_mut())
//!             .col(string_uniq(Users::Email).borrow_mut())
//!             .col(string(Users::Password).borrow_mut())
//!             .col(string(Users::Name).borrow_mut())
//!             .col(string_null(Users::ResetToken).borrow_mut())
//!             .col(timestamp_null(Users::ResetSentAt).borrow_mut())
//!             .to_owned();
//!         manager.create_table(table).await?;
//!         Ok(())
//!     }
//!
//!     async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
//!         manager
//!             .drop_table(Table::drop().table(Users::Table).to_owned())
//!             .await
//!     }
//! }
//!
//! #[derive(Iden)]
//! pub enum Users {
//!     Table,
//!     Id,
//!     Pid,
//!     Email,
//!     Name,
//!     Password,
//!     ResetToken,
//!     ResetSentAt,
//! }
//! ```

use sea_orm::sea_query::{ColumnDef, Expr, IntoIden, Table, TableCreateStatement};
use sea_orm_migration::{prelude::Iden, sea_query};

#[derive(Iden)]
enum GeneralIds {
    CreatedAt,
    UpdatedAt,
}

/// Wrapping  table schema creation.
pub fn table_auto<T>(name: T) -> TableCreateStatement
where
    T: IntoIden + 'static,
{
    timestamps(Table::create().table(name).if_not_exists().clone())
}

/// Create a primary key column with auto-increment feature.
pub fn pk_auto<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name)
        .integer()
        .not_null()
        .auto_increment()
        .primary_key()
        .clone()
}

/// Create a UUID column definition with a unique constraint.
pub fn uuid<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).unique_key().uuid().not_null().clone()
}

/// Add timestamp columns (`CreatedAt` and `UpdatedAt`) to an existing table.
#[must_use]
pub fn timestamps(t: TableCreateStatement) -> TableCreateStatement {
    let mut t = t;
    t.col(
        ColumnDef::new(GeneralIds::CreatedAt)
            .date_time()
            .not_null()
            .clone()
            .default(Expr::current_timestamp()),
    )
    .col(timestamp(GeneralIds::UpdatedAt).default(Expr::current_timestamp()));
    t.clone()
}

/// Create a nullable timestamp column definition.
pub fn timestamp_null<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).date_time().clone()
}

/// Create a non-nullable timestamp column definition.
pub fn timestamp<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).date_time().not_null().clone()
}

/// Create a non-nullable date column definition.
pub fn date<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).date().not_null().clone()
}

/// Create a nullable date column definition.
pub fn date_null<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).date().clone()
}

/// Create a non-nullable tiny integer column definition.
pub fn tiny_integer<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).tiny_integer().not_null().clone()
}

/// Create a nullable tiny integer column definition.
pub fn tiny_integer_null<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).tiny_integer().clone()
}

/// Create a unique tiny integer column definition.
pub fn tiny_integer_uniq<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).tiny_integer().unique_key().clone()
}

/// Create a non-nullable small integer column definition.
pub fn small_integer<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).small_integer().not_null().clone()
}

/// Create a nullable small integer column definition.
pub fn small_integer_null<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).small_integer().clone()
}

/// Create a unique small integer column definition.
pub fn small_integer_uniq<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).small_integer().unique_key().clone()
}

/// Create a non-nullable integer column definition.
pub fn integer<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).integer().not_null().clone()
}

/// Create a nullable integer column definition.
pub fn integer_null<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).integer().clone()
}

/// Create a unique integer column definition.
pub fn integer_uniq<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).integer().unique_key().clone()
}

/// Create a non-nullable big integer column definition.
pub fn big_integer<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).big_integer().not_null().clone()
}

/// Create a nullable big integer column definition.
pub fn big_integer_null<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).big_integer().clone()
}

/// Create a unique big integer column definition.
pub fn big_integer_uniq<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).big_integer().unique_key().clone()
}

/// Create a non-nullable float column definition.
pub fn float<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).float().not_null().clone()
}

/// Create a nullable float column definition.
pub fn float_null<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).float().clone()
}

/// Create a non-nullable double column definition.
pub fn double<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).double().not_null().clone()
}

/// Create a nullable double column definition.
pub fn double_null<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).double().clone()
}

/// Create a non-nullable decimal column definition.
pub fn decimal<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).decimal().not_null().clone()
}

/// Create a nullable decimal column definition.
pub fn decimal_null<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).decimal().clone()
}

/// Create a non-nullable decimal length column definition with custom precision and scale.
pub fn decimal_len<T>(name: T, precision: u32, scale: u32) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).decimal_len(precision, scale).not_null().clone()
}

/// Create a nullable decimal length column definition with custom precision and scale.
pub fn decimal_len_null<T>(name: T, precision: u32, scale: u32) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).decimal_len(precision, scale).clone()
}

/// Create a unique string column definition.
pub fn string_uniq<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    string(name).unique_key().clone()
}

/// Create a nullable string column definition.
pub fn string_null<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).string().clone()
}

/// Create a nullable text column definition.
pub fn text<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).text().clone()
}

/// Create a non-nullable string column definition.
pub fn string<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    string_null(name).not_null().clone()
}

/// Create a nullable boolean column definition.
pub fn bool_null<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).boolean().clone()
}

/// Create a non-nullable boolean column definition.
pub fn bool<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    bool_null(name).not_null().clone()
}
