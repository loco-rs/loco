impl Model {
    /// Counts users whose account was created at or after `since`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn count_registered_since(
        db: &DatabaseConnection,
        since: DateTimeWithTimeZone,
    ) -> ModelResult<u64> {
        let count = users::Entity::find()
            .filter(users::Column::CreatedAt.gte(since))
            .count(db)
            .await?;
        Ok(count)
    }
}
