impl Model {
    /// Counts every registered user.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn count_all(db: &DatabaseConnection) -> ModelResult<u64> {
        Ok(users::Entity::find().count(db).await?)
    }
}
