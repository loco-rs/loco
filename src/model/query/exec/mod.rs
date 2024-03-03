//! # Query Execution Builder
//!
//! This module provides a builder pattern for executing `sea_orm` queries with
//! optional conditions and pagination. The primary purpose is to simplify the
//! construction of queries by chaining method calls for conditions and
//! pagination settings.
//!
//! ## Example
//!
//! ```
//! use loco_rs::tests_cfg::db::*;
//! use sea_orm::EntityTrait;
//! use loco_rs::prelude::model::*;
//!
//! pub async fn data() {
//!     let db = dummy_connection().await;
//!     let condition_builder = query::dsl::condition().eq(test_db::Column::Name, "loco");
//!     let res = query::exec::<test_db::Entity>(&db).condition_builder(condition_builder).page(1).page_size(100).paginate().await;
//! }
//! ```
pub mod pagination;
use sea_orm::{DatabaseConnection, EntityTrait};
use std::marker::PhantomData;

use super::dsl;
use crate::{
    model::query::{PaginatedResponse, PaginationQuery},
    Result as LocoResult,
};

pub struct ExecBuilder<'a, E>
where
    E: EntityTrait,
    <E as EntityTrait>::Model: Sync,
{
    db: &'a DatabaseConnection,
    condition_builder: Option<dsl::ConditionBuilder>,
    pagination: PaginationQuery,
    marker: PhantomData<E>,
}

#[must_use]
pub fn exec<E>(db: &DatabaseConnection) -> ExecBuilder<'_, E>
where
    E: EntityTrait,
    <E as EntityTrait>::Model: Sync,
{
    ExecBuilder {
        db,
        condition_builder: None,
        pagination: PaginationQuery::default(),
        marker: PhantomData,
    }
}

/// Execute `sea_orm` query
///
/// # Examples
/// ```
/// use loco_rs::tests_cfg::db::*;
/// use sea_orm::EntityTrait;
/// use loco_rs::prelude::model::*;
/// pub async fn data() {
///   let db = dummy_connection().await;
///   let condition_builder = query::dsl::condition().eq(test_db::Column::Name, "loco");  
///   let res = query::exec::<test_db::Entity>(&db).condition_builder(condition_builder).page(1).page_size(100).paginate().await;
/// }
/// ```
impl<'a, E> ExecBuilder<'a, E>
where
    E: EntityTrait,
    <E as EntityTrait>::Model: Sync,
{
    /// Set the query DSL condition builder for filtering results.
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::model::*;
    /// async fn example() {
    ///     let db = dummy_connection().await;
    ///     let condition_builder = query::dsl::condition().eq(test_db::Column::Name, "loco");
    ///     let res = query::exec::<test_db::Entity>(&db).condition_builder(condition_builder).paginate().await;;
    /// }
    /// ```
    pub fn condition_builder(mut self, condition_builder: dsl::ConditionBuilder) -> Self {
        self.condition_builder = Some(condition_builder);
        self
    }

    /// Set the page number for pagination.
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::model::*;
    /// async fn example() {
    ///     let db = dummy_connection().await;
    ///     let res = query::exec::<test_db::Entity>(&db).page(1).paginate().await;;
    /// }
    /// ```
    pub fn page(mut self, page: u64) -> Self {
        self.pagination.page = page;
        self
    }

    /// Set the page size for pagination.
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::model::*;
    /// async fn example() {
    ///     let db = dummy_connection().await;
    ///     let res = query::exec::<test_db::Entity>(&db).page_size(100).paginate().await;;
    /// }
    /// ```
    pub fn page_size(mut self, page_size: u64) -> Self {
        self.pagination.page_size = page_size;
        self
    }

    ///  Execute the `sea_orm` query with optional conditions and pagination.
    ///
    /// # Examples
    /// ```
    /// use loco_rs::tests_cfg::db::*;
    /// use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};
    /// use loco_rs::prelude::model::*;
    /// async fn example() {
    ///     let db = dummy_connection().await;
    ///     let condition_builder = query::dsl::condition().eq(test_db::Column::Name, "loco");
    ///     let res = query::exec::<test_db::Entity>(&db).condition_builder(condition_builder).page(2).page_size(100).paginate().await;;
    /// }
    /// ```
    pub async fn paginate(&self) -> LocoResult<PaginatedResponse<E::Model>> {
        let filters = self
            .condition_builder
            .as_ref()
            .map_or_else(|| None, |condition_builder| Some(condition_builder.build()));

        let notes_entity = E::find();
        let paginated_response =
            pagination::paginate::<E>(self.db, notes_entity, filters, &self.pagination).await?;

        Ok(paginated_response)
    }
}
