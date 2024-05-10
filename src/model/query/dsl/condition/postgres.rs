use sea_orm::{
    prelude::Expr,
    sea_query::{extension::postgres::PgExpr, IntoCondition},
    ColumnTrait, Condition,
};

use crate::model::query::dsl::condition::ConditionBuilderTrait;
pub struct Postgres {
    condition: Condition,
}

impl From<Postgres> for Condition {
    fn from(postgres: Postgres) -> Self {
        postgres.condition
    }
}
impl ConditionBuilderTrait for Postgres {
    fn new(condition: Condition) -> Self {
        Self { condition }
    }

    fn get_condition(&self) -> &Condition {
        &self.condition
    }

    fn ilike<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        Self::with(
            self.condition.add(
                Expr::col((col.entity_name(), col))
                    .ilike(a)
                    .into_condition(),
            ),
        )
    }

    fn not_ilike<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        Self::with(
            self.condition.add(
                Expr::col((col.entity_name(), col))
                    .not_ilike(a)
                    .into_condition(),
            ),
        )
    }

    fn ilike_starts_with<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        let expr = Expr::col((col.entity_name(), col)).ilike(format!("{}%", a.into()));
        Self::with(self.condition.add(expr))
    }

    fn ilike_ends_with<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        let expr = Expr::col((col.entity_name(), col)).ilike(format!("%{}", a.into()));
        Self::with(self.condition.add(expr))
    }

    fn ilike_contains<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        let expr = Expr::col((col.entity_name(), col)).ilike(format!("%{}%", a.into()));
        Self::with(self.condition.add(expr))
    }
}
#[cfg(test)]
mod tests {

    use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};

    use super::*;
    use crate::tests_cfg::db::*;

    #[test]
    fn postgres_condition_eq() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(Postgres::condition().eq(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" = 1"
        );
    }

    #[test]
    fn postgres_condition_ne() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .ne(test_db::Column::Name, "loco")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" <> 'loco'"
        );
    }

    #[test]
    fn postgres_condition_gt() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(Postgres::condition().gt(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" > 1"
        );
    }

    #[test]
    fn postgres_condition_gte() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(Postgres::condition().gte(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" >= 1"
        );
    }

    #[test]
    fn postgres_condition_lt() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(Postgres::condition().lt(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" < 1"
        );
    }

    #[test]
    fn postgres_condition_lte() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(Postgres::condition().lte(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" <= 1"
        );
    }

    #[test]
    fn postgres_condition_between() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .between(test_db::Column::Id, 1, 2)
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" BETWEEN 1 AND 2"
        );
    }

    #[test]
    fn postgres_condition_not_between() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .not_between(test_db::Column::Id, 1, 2)
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" NOT BETWEEN 1 AND 2"
        );
    }

    #[test]
    fn postgres_condition_like() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .like(test_db::Column::Name, "%lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" LIKE '%lo'"
        );
    }
    #[test]
    fn postgres_condition_ilike() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .ilike(test_db::Column::Name, "%Lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" ILIKE '%Lo'"
        );
    }
    #[test]
    fn postgres_condition_not_like() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .not_like(test_db::Column::Name, "%lo%")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" NOT LIKE '%lo%'"
        );
    }

    #[test]
    fn postgres_condition_not_ilike() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .not_ilike(test_db::Column::Name, "%Lo%")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" NOT ILIKE '%Lo%'"
        );
    }

    #[test]
    fn postgres_condition_starts_with() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .starts_with(test_db::Column::Name, "lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" LIKE 'lo%'"
        );
    }

    #[test]
    fn postgres_condition_ilike_starts_with() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .ilike_starts_with(test_db::Column::Name, "Lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" ILIKE 'Lo%'"
        );
    }

    #[test]
    fn postgres_condition_ends_with() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .ends_with(test_db::Column::Name, "lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" LIKE '%lo'"
        );
    }

    #[test]
    fn postgres_condition_ilike_ends_with() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .ilike_ends_with(test_db::Column::Name, "lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" ILIKE '%lo'"
        );
    }

    #[test]
    fn postgres_condition_contains() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .contains(test_db::Column::Name, "lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" LIKE '%lo%'"
        );
    }

    #[test]
    fn postgres_condition_ilike_contains() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .ilike_contains(test_db::Column::Name, "lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" ILIKE '%lo%'"
        );
    }

    #[test]
    fn postgres_condition_is_null() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(Postgres::condition().is_null(test_db::Column::Name).build())
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" IS NULL"
        );
    }

    #[test]
    fn postgres_condition_is_not_null() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .is_not_null(test_db::Column::Name)
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" IS NOT NULL"
        );
    }

    #[test]
    fn postgres_condition_is_in() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .is_in(test_db::Column::Id, [1])
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" IN (1)"
        );
    }

    #[test]
    fn postgres_condition_is_not_in() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Postgres::condition()
                    .is_not_in(test_db::Column::Id, [1])
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" NOT IN (1)"
        );
    }
}
