impl Model {
    /// Deletes unverified users registered more than `days` ago, returning how
    /// many rows went.
    ///
    /// # Errors
    ///
    /// When the delete fails.
    pub async fn purge_unverified_older_than(
        db: &DatabaseConnection,
        days: i64,
    ) -> ModelResult<u64> {
        let cutoff = Local::now() - Duration::days(days);

        let result = users::Entity::delete_many()
            .filter(users::Column::EmailVerifiedAt.is_null())
            .filter(users::Column::CreatedAt.lt(cutoff))
            .exec(db)
            .await?;

        Ok(result.rows_affected)
    }
}
