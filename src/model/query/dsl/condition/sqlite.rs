use sea_orm::{
    prelude::Expr,
    sea_query::{Func, IntoCondition},
    ColumnTrait, Condition,
};

use crate::model::query::dsl::condition::ConditionBuilderTrait;

pub struct Sqlite {
    condition: Condition,
}

impl From<Sqlite> for Condition {
    fn from(sqlite: Sqlite) -> Self {
        sqlite.condition
    }
}

impl ConditionBuilderTrait for Sqlite {
    fn new(condition: Condition) -> Self {
        Self { condition }
    }

    fn get_condition(&self) -> &Condition {
        &self.condition
    }

    fn ilike<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        let expr = Expr::expr(Func::lower(Expr::col(col))).like(a.into().to_lowercase());
        Self::with(self.condition.add(expr.into_condition()))
    }

    fn not_ilike<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        let expr = Expr::expr(Func::lower(Expr::col(col))).not_like(a.into().to_lowercase());
        Self::with(self.condition.add(expr.into_condition()))
    }

    fn ilike_starts_with<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        let expr =
            Expr::expr(Func::lower(Expr::col(col))).like(format!("{}%", a.into().to_lowercase()));
        Self::with(self.condition.add(expr.into_condition()))
    }

    fn ilike_ends_with<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        let expr =
            Expr::expr(Func::lower(Expr::col(col))).like(format!("%{}", a.into().to_lowercase()));
        Self::with(self.condition.add(expr.into_condition()))
    }

    fn ilike_contains<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        let expr =
            Expr::expr(Func::lower(Expr::col(col))).like(format!("%{}%", a.into().to_lowercase()));
        Self::with(self.condition.add(expr.into_condition()))
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};

    use crate::{
        model::query::dsl::condition::{sqlite::Sqlite, ConditionBuilderTrait},
        tests_cfg::db::*,
    };
    #[test]
    fn sqlite_condition_eq() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(Sqlite::condition().eq(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" = 1"
        );
    }

    #[test]
    fn sqlite_condition_ne() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .ne(test_db::Column::Name, "loco")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" <> 'loco'"
        );
    }

    #[test]
    fn sqlite_condition_gt() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(Sqlite::condition().gt(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" > 1"
        );
    }

    #[test]
    fn sqlite_condition_gte() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(Sqlite::condition().gte(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" >= 1"
        );
    }

    #[test]
    fn sqlite_condition_lt() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(Sqlite::condition().lt(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" < 1"
        );
    }

    #[test]
    fn sqlite_condition_lte() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(Sqlite::condition().lte(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" <= 1"
        );
    }

    #[test]
    fn sqlite_condition_between() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .between(test_db::Column::Id, 1, 2)
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" BETWEEN 1 AND 2"
        );
    }

    #[test]
    fn sqlite_condition_not_between() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .not_between(test_db::Column::Id, 1, 2)
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" NOT BETWEEN 1 AND 2"
        );
    }

    #[test]
    fn sqlite_condition_like() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .like(test_db::Column::Name, "%lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" LIKE '%lo'"
        );
    }

    #[test]
    fn sqlite_condition_ilike() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .ilike(test_db::Column::Name, "%Lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE LOWER(\"name\") LIKE '%lo'"
        );
    }

    #[test]
    fn sqlite_condition_not_like() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .not_like(test_db::Column::Name, "%lo%")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" NOT LIKE '%lo%'"
        );
    }

    #[test]
    fn sqlite_condition_not_ilike() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .not_ilike(test_db::Column::Name, "%Lo%")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE LOWER(\"name\") NOT LIKE '%lo%'"
        );
    }

    #[test]
    fn sqlite_condition_starts_with() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .starts_with(test_db::Column::Name, "lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" LIKE 'lo%'"
        );
    }

    #[test]
    fn sqlite_condition_ilike_starts_with() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .ilike_starts_with(test_db::Column::Name, "Lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE LOWER(\"name\") LIKE 'lo%'"
        );
    }

    #[test]
    fn sqlite_condition_ends_with() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .ends_with(test_db::Column::Name, "lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" LIKE '%lo'"
        );
    }

    #[test]
    fn sqlite_condition_ilike_ends_with() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .ilike_ends_with(test_db::Column::Name, "Lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE LOWER(\"name\") LIKE '%lo'"
        );
    }

    #[test]
    fn sqlite_condition_contains() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .contains(test_db::Column::Name, "lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" LIKE '%lo%'"
        );
    }

    #[test]
    fn sqlite_condition_ilike_contains() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .ilike_contains(test_db::Column::Name, "lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE LOWER(\"name\") LIKE '%lo%'"
        );
    }

    #[test]
    fn sqlite_condition_is_null() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(Sqlite::condition().is_null(test_db::Column::Name).build())
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" IS NULL"
        );
    }

    #[test]
    fn sqlite_condition_is_not_null() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .is_not_null(test_db::Column::Name)
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"name\" IS NOT NULL"
        );
    }

    #[test]
    fn sqlite_condition_is_in() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(Sqlite::condition().is_in(test_db::Column::Id, [1]).build())
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" IN (1)"
        );
    }

    #[test]
    fn sqlite_condition_is_not_in() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                Sqlite::condition()
                    .is_not_in(test_db::Column::Id, [1])
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::Sqlite)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"id\" NOT IN (1)"
        );
    }
}
