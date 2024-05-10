use sea_orm::{ColumnTrait, Condition, Value};
pub mod mysql;
pub mod postgres;
pub mod sqlite;

use crate::model::query::dsl::date_range::DateRangeBuilder;

#[must_use]
pub trait ConditionBuilderTrait: Sized + Into<Condition> {
    fn new(condition: Condition) -> Self;
    fn get_condition(&self) -> &Condition;
    fn condition() -> Self {
        Self::new(Condition::all())
    }
    fn with(condition: Condition) -> Self {
        Self::new(condition)
    }

    /// where condition the given column equals the given value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().eq(test_db::Column::Id, 1).build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" = 1"
    ///     );
    /// ````
    ///
    /// On string field
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().eq(test_db::Column::Name, "loco").build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" = 'loco'"
    ///     );
    /// ````
    #[must_use]
    fn eq<T: ColumnTrait, V: Into<Value>>(self, col: T, value: V) -> Self {
        Self::with(self.into().add(col.eq(value)))
    }

    /// where condition the given column not equals the given value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().ne(test_db::Column::Id, 1).build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" <> 1"
    ///     );
    /// ````
    #[must_use]
    fn ne<T: ColumnTrait, V: Into<Value>>(self, col: T, value: V) -> Self {
        Self::with(self.into().add(col.ne(value)))
    }

    /// where condition the given column greater than the given value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().gt(test_db::Column::Id, 1).build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" > 1"
    ///     );
    /// ````
    #[must_use]
    fn gt<T: ColumnTrait, V: Into<Value>>(self, col: T, value: V) -> Self {
        Self::with(self.into().add(col.gt(value)))
    }

    /// where condition the given column greater than or equal to the given
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().gte(test_db::Column::Id, 1).build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" >= 1"
    ///     );
    /// ````
    #[must_use]
    fn gte<T: ColumnTrait, V: Into<Value>>(self, col: T, value: V) -> Self {
        Self::with(self.into().add(col.gte(value)))
    }

    /// where condition the given column smaller than to the given
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    ///
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().lt(test_db::Column::Id, 1).build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" < 1"
    ///     );
    /// ````
    #[must_use]
    fn lt<T: ColumnTrait, V: Into<Value>>(self, col: T, value: V) -> Self {
        Self::with(self.into().add(col.lt(value)))
    }

    /// where condition the given column smaller than or equal to the given
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    ///
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().lte(test_db::Column::Id, 1).build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" <= 1"
    ///     );
    /// ````
    #[must_use]
    fn lte<T: ColumnTrait, V: Into<Value>>(self, col: T, value: V) -> Self {
        Self::with(self.into().add(col.lte(value)))
    }

    /// where condition the given column between the given values
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    ///
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().between(test_db::Column::Id, 1, 2).build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" BETWEEN 1 AND 2"
    ///     );
    /// ````
    #[must_use]
    fn between<T: ColumnTrait, V: Into<Value>>(self, col: T, a: V, b: V) -> Self {
        Self::with(self.into().add(col.between(a, b)))
    }

    /// where condition the given column not between the given values
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().not_between(test_db::Column::Id, 1, 2).build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" NOT BETWEEN 1 AND 2"
    ///     );
    /// ````
    #[must_use]
    fn not_between<T: ColumnTrait, V: Into<Value>>(self, col: T, a: V, b: V) -> Self {
        Self::with(self.into().add(col.not_between(a, b)))
    }

    /// where condition the given column like given values
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().like(test_db::Column::Name, "%lo").build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" LIKE '%lo'"
    ///     );
    /// ````
    #[must_use]
    fn like<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        Self::with(self.into().add(col.like(a)))
    }

    /// where condition the given column ilike given values
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().ilike(test_db::Column::Name, "%Lo").build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    /// assert_eq!(
    ///    query_str,
    ///   "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" ILIKE '%Lo'"
    /// );
    #[must_use]
    fn ilike<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self;

    /// where condition the given column not like given values
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().not_like(test_db::Column::Name, "%lo").build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" NOT LIKE '%lo'"
    ///     );
    /// ````
    #[must_use]
    fn not_like<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        Self::with(self.into().add(col.not_like(a)))
    }

    /// where condition the given column not ilike given values
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().not_ilike(test_db::Column::Name, "%Lo").build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    /// assert_eq!(
    ///    query_str,
    ///   "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" NOT ILIKE '%Lo'"
    /// );
    #[must_use]
    fn not_ilike<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self;

    /// where condition the given column start with given values
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().starts_with(test_db::Column::Name, "lo").build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" LIKE 'lo%'"
    ///     );
    /// ````
    #[must_use]
    fn starts_with<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        Self::with(self.into().add(col.starts_with(a)))
    }

    /// where condition the given column start with ilike given values
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    ///
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().ilike_starts_with(test_db::Column::Name, "lo").build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///    assert_eq!(
    ///       query_str,
    ///      "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" ILIKE 'lo%'"
    ///     );
    /// ```
    #[must_use]
    fn ilike_starts_with<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self;

    /// where condition the given column end with given values
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().ends_with(test_db::Column::Name, "lo").build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" LIKE '%lo'"
    ///     );
    /// ````
    #[must_use]
    fn ends_with<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        Self::with(self.into().add(col.ends_with(a)))
    }

    /// where condition the given column end with ilike given values
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait}; let query_str = test_db::Entity::find()
    ///                 .select_only()
    ///                 .column(test_db::Column::Id)
    ///                 .filter(Postgres::condition().ilike_ends_with(test_db::Column::Name, "lo").build())
    ///                 .build(sea_orm::DatabaseBackend::Postgres)
    ///                 .to_string();
    ///
    ///    assert_eq!(
    ///      query_str,
    ///     "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" ILIKE '%lo'"
    ///    );
    /// ```
    #[must_use]
    fn ilike_ends_with<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self;

    /// where condition the given column end with given values
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().contains(test_db::Column::Name, "lo").build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" LIKE '%lo%'"
    ///     );
    /// ````
    #[must_use]
    fn contains<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        Self::with(self.into().add(col.contains(a)))
    }

    /// where condition the given column contains ilike given values
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///                .select_only()
    ///                .column(test_db::Column::Id)
    ///                .filter(Postgres::condition().ilike_contains(test_db::Column::Name, "lo").build())
    ///                .build(sea_orm::DatabaseBackend::Postgres)
    ///                .to_string();
    ///
    ///   assert_eq!(
    ///     query_str,
    ///     "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" ILIKE '%lo%'"
    ///     );
    #[must_use]
    fn ilike_contains<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self;

    /// where condition the given column is null
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().is_null(test_db::Column::Name).build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" IS NULL"
    ///     );
    /// ````
    #[must_use]
    #[allow(clippy::wrong_self_convention)]
    fn is_null<T: ColumnTrait>(self, col: T) -> Self {
        Self::with(self.into().add(col.is_null()))
    }

    /// where condition the given column is not null
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().is_not_null(test_db::Column::Name).build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" IS NOT NULL"
    ///     );
    /// ````
    #[must_use]
    #[allow(clippy::wrong_self_convention)]
    fn is_not_null<T: ColumnTrait>(self, col: T) -> Self {
        Self::with(self.into().add(col.is_not_null()))
    }

    /// where condition the given column is in
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().is_in(test_db::Column::Id, [1]).build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" IN (1)"
    ///     );
    /// ````
    #[must_use]
    #[allow(clippy::wrong_self_convention)]
    fn is_in<T: ColumnTrait, V: Into<Value>, I: IntoIterator<Item = V>>(
        self,
        col: T,
        values: I,
    ) -> Self {
        Self::with(self.into().add(col.is_in(values)))
    }

    /// where condition the given column is not in
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let query_str = test_db::Entity::find()
    ///         .select_only()
    ///         .column(test_db::Column::Id)
    ///         .filter(Postgres::condition().is_not_in(test_db::Column::Id, [1]).build())
    ///         .build(sea_orm::DatabaseBackend::Postgres)
    ///         .to_string();
    ///
    ///     assert_eq!(
    ///         query_str,
    ///         "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" NOT IN (1)"
    ///     );
    /// ````
    #[must_use]
    #[allow(clippy::wrong_self_convention)]
    fn is_not_in<T: ColumnTrait, V: Into<Value>, I: IntoIterator<Item = V>>(
        self,
        col: T,
        values: I,
    ) -> Self {
        Self::with(self.into().add(col.is_not_in(values)))
    }

    /// where condition the given column is not null
    /// value
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::*;
    /// use loco_rs::prelude::query::condition::{postgres::Postgres, ConditionBuilderTrait};
    /// let from_date = chrono::NaiveDateTime::parse_from_str("2024-03-01
    /// 22:10:57", "%Y-%m-%d %H:%M:%S").unwrap(); let to_date =
    /// chrono::NaiveDateTime::parse_from_str("2024-03-25 22:10:57", "%Y-%m-%d
    /// %H:%M:%S").unwrap();
    ///
    /// let condition = Postgres::condition()
    ///     .date_range(test_db::Column::CreatedAt)
    ///     .dates(Some(&from_date), Some(&to_date))
    ///     .build();
    ///
    /// let query_str = test_db::Entity::find()
    ///     .select_only()
    ///     .column(test_db::Column::Id)
    ///     .filter(condition.build())
    ///     .build(sea_orm::DatabaseBackend::Postgres)
    ///     .to_string();
    ///
    /// assert_eq!(
    ///     query_str,
    ///     "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"created_at\" BETWEEN '2024-03-01 22:10:57' AND '2024-03-25 22:10:57'" );
    /// ````
    #[must_use]
    fn date_range<T: ColumnTrait>(self, col: T) -> DateRangeBuilder<T, Self> {
        DateRangeBuilder::new(self, col)
    }

    #[must_use]
    fn build(&self) -> Condition {
        self.get_condition().clone()
    }
}
