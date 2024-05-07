use sea_orm::{
    prelude::Expr,
    sea_query::{extension::postgres::PgExpr, IntoCondition},
    ColumnTrait, Condition,
};

use crate::model::query::dsl::{
    condition::ConditionBuilderTrait, date_range, date_range::DateRangeBuilder,
};

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

    fn date_range<T: ColumnTrait>(self, col: T) -> DateRangeBuilder<T, Self> {
        date_range::DateRangeBuilder::new(self, col)
    }
}

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
        todo!()
    }

    fn get_condition(&self) -> &Condition {
        todo!()
    }

    fn ilike<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        todo!()
    }

    fn not_ilike<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        todo!()
    }

    fn ilike_starts_with<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        todo!()
    }

    fn ilike_ends_with<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        todo!()
    }

    fn ilike_contains<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        todo!()
    }

    fn date_range<T: ColumnTrait>(self, col: T) -> DateRangeBuilder<T, Self> {
        todo!()
    }
}

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
        todo!()
    }

    fn not_ilike<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        todo!()
    }

    fn ilike_starts_with<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        todo!()
    }

    fn ilike_ends_with<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        todo!()
    }

    fn ilike_contains<T: ColumnTrait, V: Into<String>>(self, col: T, a: V) -> Self {
        todo!()
    }

    fn date_range<T: ColumnTrait>(self, col: T) -> DateRangeBuilder<T, Self> {
        todo!()
    }
}
