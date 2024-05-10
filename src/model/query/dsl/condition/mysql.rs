use sea_orm::{
    prelude::Expr,
    sea_query::{Func, IntoCondition},
    ColumnTrait, Condition,
};

use crate::model::query::dsl::condition::ConditionBuilderTrait;

pub struct MySql {
    condition: Condition,
}

impl From<MySql> for Condition {
    fn from(mysql: MySql) -> Self {
        mysql.condition
    }
}

impl ConditionBuilderTrait for MySql {
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
        model::query::dsl::condition::{mysql::MySql, ConditionBuilderTrait},
        tests_cfg::db::*,
    };
    #[test]
    fn mysql_condition_eq() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(MySql::condition().eq(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`id` = 1"
        );
    }

    #[test]
    fn mysql_condition_ne() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(MySql::condition().ne(test_db::Column::Name, "loco").build())
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`name` <> 'loco'"
        );
    }

    #[test]
    fn mysql_condition_gt() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(MySql::condition().gt(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`id` > 1"
        );
    }

    #[test]
    fn mysql_condition_gte() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(MySql::condition().gte(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`id` >= 1"
        );
    }

    #[test]
    fn mysql_condition_lt() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(MySql::condition().lt(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`id` < 1"
        );
    }

    #[test]
    fn mysql_condition_lte() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(MySql::condition().lte(test_db::Column::Id, 1).build())
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`id` <= 1"
        );
    }

    #[test]
    fn mysql_condition_between() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .between(test_db::Column::Id, 1, 2)
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`id` BETWEEN 1 AND 2"
        );
    }

    #[test]
    fn mysql_condition_not_between() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .not_between(test_db::Column::Id, 1, 2)
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`id` NOT BETWEEN 1 AND 2"
        );
    }

    #[test]
    fn mysql_condition_like() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .like(test_db::Column::Name, "%lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`name` LIKE '%lo'"
        );
    }

    #[test]
    fn mysql_condition_ilike() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .ilike(test_db::Column::Name, "%Lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE LOWER(`name`) LIKE '%lo'"
        );
    }

    #[test]
    fn mysql_condition_not_like() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .not_like(test_db::Column::Name, "%lo%")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`name` NOT LIKE '%lo%'"
        );
    }

    #[test]
    fn mysql_condition_not_ilike() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .not_ilike(test_db::Column::Name, "%Lo%")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE LOWER(`name`) NOT LIKE '%lo%'"
        );
    }

    #[test]
    fn mysql_condition_starts_with() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .starts_with(test_db::Column::Name, "lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`name` LIKE 'lo%'"
        );
    }

    #[test]
    fn mysql_condition_ilike_starts_with() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .ilike_starts_with(test_db::Column::Name, "Lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE LOWER(`name`) LIKE 'lo%'"
        );
    }

    #[test]
    fn mysql_condition_ends_with() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .ends_with(test_db::Column::Name, "lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`name` LIKE '%lo'"
        );
    }

    #[test]
    fn mysql_condition_ilike_ends_with() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .ilike_ends_with(test_db::Column::Name, "Lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE LOWER(`name`) LIKE '%lo'"
        );
    }

    #[test]
    fn mysql_condition_contains() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .contains(test_db::Column::Name, "lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`name` LIKE '%lo%'"
        );
    }

    #[test]
    fn mysql_condition_ilike_contains() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .ilike_contains(test_db::Column::Name, "lo")
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE LOWER(`name`) LIKE '%lo%'"
        );
    }

    #[test]
    fn mysql_condition_is_null() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(MySql::condition().is_null(test_db::Column::Name).build())
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`name` IS NULL"
        );
    }

    #[test]
    fn mysql_condition_is_not_null() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .is_not_null(test_db::Column::Name)
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`name` IS NOT NULL"
        );
    }

    #[test]
    fn mysql_condition_is_in() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(MySql::condition().is_in(test_db::Column::Id, [1]).build())
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`id` IN (1)"
        );
    }

    #[test]
    fn mysql_condition_is_not_in() {
        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(
                MySql::condition()
                    .is_not_in(test_db::Column::Id, [1])
                    .build(),
            )
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT `loco`.`id` FROM `loco` WHERE `loco`.`id` NOT IN (1)"
        );
    }
}
