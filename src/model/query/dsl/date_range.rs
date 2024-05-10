use chrono::NaiveDateTime;
use sea_orm::ColumnTrait;

use crate::model::query::dsl::condition::ConditionBuilderTrait;
pub struct DateRangeBuilder<T: ColumnTrait, U: ConditionBuilderTrait> {
    col: T,
    condition_builder: U,
    from_date: Option<NaiveDateTime>,
    to_date: Option<NaiveDateTime>,
}

impl<T: ColumnTrait, U: ConditionBuilderTrait> DateRangeBuilder<T, U> {
    pub const fn new(condition_builder: U, col: T) -> Self {
        Self {
            col,
            condition_builder,
            from_date: None,
            to_date: None,
        }
    }

    #[must_use]
    pub fn dates(self, from: Option<&NaiveDateTime>, to: Option<&NaiveDateTime>) -> Self {
        Self {
            col: self.col,
            condition_builder: self.condition_builder,
            from_date: from.copied(),
            to_date: to.copied(),
        }
    }

    #[must_use]
    pub fn from(self, from: &NaiveDateTime) -> Self {
        Self {
            col: self.col,
            condition_builder: self.condition_builder,
            from_date: Some(*from),
            to_date: self.to_date,
        }
    }

    #[must_use]
    pub fn to(self, to: &NaiveDateTime) -> Self {
        Self {
            col: self.col,
            condition_builder: self.condition_builder,
            from_date: self.from_date,
            to_date: Some(*to),
        }
    }

    pub fn build(self) -> U {
        let con = match (self.from_date, self.to_date) {
            (None, None) => self.condition_builder.into(),
            (None, Some(to)) => self.condition_builder.into().add(self.col.lt(to)),
            (Some(from), None) => self.condition_builder.into().add(self.col.gt(from)),
            (Some(from), Some(to)) => self
                .condition_builder
                .into()
                .add(self.col.between(from, to)),
        };
        U::new(con)
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{EntityTrait, QueryFilter, QuerySelect, QueryTrait};

    use crate::{
        model::query::dsl::condition::{postgres::Postgres, ConditionBuilderTrait},
        tests_cfg::db::*,
    };

    #[test]
    fn condition_date_range_from() {
        let date =
            chrono::NaiveDateTime::parse_from_str("2024-03-01 22:10:57", "%Y-%m-%d %H:%M:%S")
                .unwrap();

        let condition = Postgres::condition()
            .date_range(test_db::Column::CreatedAt)
            .from(&date)
            .build();

        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(condition.build())
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"created_at\" > '2024-03-01 \
             22:10:57'"
        );
    }

    #[test]
    fn condition_date_range_to() {
        let date =
            chrono::NaiveDateTime::parse_from_str("2024-03-01 22:10:57", "%Y-%m-%d %H:%M:%S")
                .unwrap();

        let condition = Postgres::condition()
            .date_range(test_db::Column::CreatedAt)
            .to(&date)
            .build();

        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(condition.build())
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"created_at\" < '2024-03-01 \
             22:10:57'"
        );
    }

    #[test]
    fn condition_date_both() {
        let from_date =
            chrono::NaiveDateTime::parse_from_str("2024-03-01 22:10:57", "%Y-%m-%d %H:%M:%S")
                .unwrap();
        let to_date =
            chrono::NaiveDateTime::parse_from_str("2024-03-25 22:10:57", "%Y-%m-%d %H:%M:%S")
                .unwrap();

        let condition = Postgres::condition()
            .date_range(test_db::Column::CreatedAt)
            .dates(Some(&from_date), Some(&to_date))
            .build();

        let query_str = test_db::Entity::find()
            .select_only()
            .column(test_db::Column::Id)
            .filter(condition.build())
            .build(sea_orm::DatabaseBackend::Postgres)
            .to_string();

        assert_eq!(
            query_str,
            "SELECT \"loco\".\"id\" FROM \"loco\" WHERE \"loco\".\"created_at\" BETWEEN \
             '2024-03-01 22:10:57' AND '2024-03-25 22:10:57'"
        );
    }
}
