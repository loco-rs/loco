use loco_rs::model::query;

impl Model {
    /// Searches users whose name or email contains `term`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn search(
        db: &DatabaseConnection,
        term: &str,
        pagination: &query::PaginationQuery,
    ) -> Result<query::PageResponse<Self>> {
        let matches = Condition::any()
            .add(users::Column::Name.contains(term))
            .add(users::Column::Email.contains(term));

        query::paginate(db, users::Entity::find(), Some(matches), pagination).await
    }
}
