//! Explicit tenant scoping for Sea-ORM entities and queries.

use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DeleteMany, EntityTrait, QueryFilter, Select,
    UpdateMany, Value,
};

use super::{ModelError, ModelResult};

/// Marks a Sea-ORM entity as tenant-owned.
///
/// Implement this trait in the entity's hand-written model module. The
/// generated entity remains untouched, while [`TenantQueryExt`] and
/// [`TenantActiveModelExt`] learn which column carries the tenant key.
///
/// Requires the opt-in `multi-tenancy` feature.
///
/// ```rust
/// use loco_rs::prelude::*;
/// # mod notes {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "notes")]
/// #     pub struct Model {
/// #         #[sea_orm(primary_key)]
/// #         pub id: i64,
/// #         pub tenant_id: i64,
/// #     }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// #     pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
///
/// impl TenantEntity for notes::Entity {
///     type TenantId = i64;
///
///     fn tenant_column() -> notes::Column {
///         notes::Column::TenantId
///     }
/// }
///
/// let query = notes::Entity::find().in_tenant(42);
/// let note: notes::ActiveModel = Default::default();
/// let note = note.set_tenant(42)?;
/// # assert_eq!(note.tenant_id, Set(42));
/// # Ok::<(), ModelError>(())
/// ```
pub trait TenantEntity: EntityTrait {
    /// The value type stored in the entity's tenant column.
    type TenantId: Into<Value>;

    /// Returns the column used to isolate this entity by tenant.
    fn tenant_column() -> Self::Column;
}

/// Adds an explicit tenant filter to tenant-owned Sea-ORM queries.
///
/// This trait is implemented for selects, bulk updates, and bulk deletes.
/// Keeping the scope explicit avoids request-global state leaking between
/// asynchronous requests and makes write isolation visible at the call site.
///
/// Only the target entity is filtered; joined or referenced rows are not scoped.
/// Callers must authorize the tenant and prevent tenant-key changes in bulk
/// update fields. Queries without [`Self::in_tenant`] remain unscoped.
pub trait TenantQueryExt: QueryFilter + Sized {
    /// The tenant-owned entity queried by this builder.
    type Entity: TenantEntity;

    /// Restricts this query to one tenant.
    #[must_use]
    fn in_tenant(self, tenant_id: <Self::Entity as TenantEntity>::TenantId) -> Self {
        self.filter(Self::Entity::tenant_column().eq(tenant_id))
    }
}

impl<E> TenantQueryExt for Select<E>
where
    E: TenantEntity,
{
    type Entity = E;
}

impl<E> TenantQueryExt for UpdateMany<E>
where
    E: TenantEntity,
{
    type Entity = E;
}

impl<E> TenantQueryExt for DeleteMany<E>
where
    E: TenantEntity,
{
    type Entity = E;
}

/// Safely assigns the tenant key on a tenant-owned Sea-ORM active model.
///
/// An unset key is assigned and an identical key is accepted. Attempting to
/// overwrite a different tenant returns [`ModelError::TenantMismatch`]. This
/// protects create flows that deserialize or otherwise pre-populate an active
/// model before the trusted tenant is applied.
pub trait TenantActiveModelExt: ActiveModelTrait + Sized
where
    Self::Entity: TenantEntity,
{
    /// Assigns the model's tenant column without allowing reassignment.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::TenantMismatch`] when the model already carries
    /// a different tenant, or [`ModelError::DbErr`] when `TenantId` does not
    /// match the declared column's value type.
    fn set_tenant(
        mut self,
        tenant_id: <Self::Entity as TenantEntity>::TenantId,
    ) -> ModelResult<Self> {
        let column = Self::Entity::tenant_column();
        let tenant_id = tenant_id.into();

        match self.get(column) {
            ActiveValue::NotSet => self.try_set(column, tenant_id)?,
            ActiveValue::Set(current) | ActiveValue::Unchanged(current) if current == tenant_id => {
            }
            ActiveValue::Set(_) | ActiveValue::Unchanged(_) => {
                return Err(ModelError::TenantMismatch);
            }
        }

        Ok(self)
    }
}

impl<A> TenantActiveModelExt for A
where
    A: ActiveModelTrait,
    A::Entity: TenantEntity,
{
}

#[cfg(test)]
mod tests {
    use sea_orm::{
        sea_query::Expr, ActiveValue::Set, ConnectOptions, ConnectionTrait, Database,
        DatabaseConnection, DbBackend, QueryOrder, QueryTrait, Schema,
    };

    use super::*;

    mod documents {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
        #[sea_orm(table_name = "documents")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i64,
            pub tenant_id: i64,
            pub title: String,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    mod malformed_documents {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
        #[sea_orm(table_name = "malformed_documents")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i64,
            pub tenant_id: i64,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    impl TenantEntity for documents::Entity {
        type TenantId = i64;

        fn tenant_column() -> documents::Column {
            documents::Column::TenantId
        }
    }

    impl TenantEntity for malformed_documents::Entity {
        type TenantId = String;

        fn tenant_column() -> malformed_documents::Column {
            malformed_documents::Column::TenantId
        }
    }

    async fn documents_db() -> DatabaseConnection {
        let mut options = ConnectOptions::new("sqlite::memory:");
        options.max_connections(1);
        let db = Database::connect(options).await.unwrap();
        let schema = Schema::new(DbBackend::Sqlite);
        db.execute(&schema.create_table_from_entity(documents::Entity))
            .await
            .unwrap();

        let documents = [(1, 42, "roadmap"), (2, 42, "notes"), (3, 7, "roadmap")]
            .into_iter()
            .map(|(id, tenant_id, title)| {
                documents::ActiveModel {
                    id: Set(id),
                    title: Set(title.to_owned()),
                    ..Default::default()
                }
                .set_tenant(tenant_id)
                .unwrap()
            });
        documents::Entity::insert_many(documents)
            .exec(&db)
            .await
            .unwrap();

        db
    }

    #[tokio::test]
    async fn scoped_reads_exclude_other_tenants_even_by_primary_key() {
        let db = documents_db().await;
        let documents = documents::Entity::find()
            .in_tenant(42)
            .order_by_asc(documents::Column::Id)
            .all(&db)
            .await
            .unwrap();

        assert_eq!(
            documents
                .iter()
                .map(|document| document.id)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        assert!(documents::Entity::find_by_id(3)
            .in_tenant(42)
            .one(&db)
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            documents::Entity::find_by_id(3)
                .in_tenant(7)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
                .title,
            "roadmap"
        );
    }

    #[tokio::test]
    async fn scoped_bulk_updates_leave_other_tenants_unchanged() {
        let db = documents_db().await;
        let denied = documents::Entity::update_many()
            .col_expr(documents::Column::Title, Expr::value("forbidden"))
            .filter(documents::Column::Id.eq(3))
            .in_tenant(42)
            .exec(&db)
            .await
            .unwrap();
        assert_eq!(denied.rows_affected, 0);

        let updated = documents::Entity::update_many()
            .col_expr(documents::Column::Title, Expr::value("released"))
            .in_tenant(42)
            .exec(&db)
            .await
            .unwrap();
        assert_eq!(updated.rows_affected, 2);

        let documents = documents::Entity::find()
            .order_by_asc(documents::Column::Id)
            .all(&db)
            .await
            .unwrap();
        assert_eq!(
            documents
                .iter()
                .map(|document| (document.id, document.tenant_id, document.title.as_str()))
                .collect::<Vec<_>>(),
            [(1, 42, "released"), (2, 42, "released"), (3, 7, "roadmap")]
        );
    }

    #[tokio::test]
    async fn scoped_bulk_deletes_preserve_other_tenants() {
        let db = documents_db().await;
        let denied = documents::Entity::delete_many()
            .filter(documents::Column::Id.eq(3))
            .in_tenant(42)
            .exec(&db)
            .await
            .unwrap();
        assert_eq!(denied.rows_affected, 0);

        let deleted = documents::Entity::delete_many()
            .in_tenant(42)
            .exec(&db)
            .await
            .unwrap();
        assert_eq!(deleted.rows_affected, 2);
        assert_eq!(
            documents::Entity::find().all(&db).await.unwrap(),
            [documents::Model {
                id: 3,
                tenant_id: 7,
                title: "roadmap".to_owned(),
            }]
        );
    }

    #[test]
    fn scopes_selects_and_composes_with_existing_filters() {
        let query = documents::Entity::find()
            .filter(documents::Column::Title.eq("roadmap"))
            .in_tenant(42)
            .build(DbBackend::Postgres)
            .to_string();

        assert_eq!(
            query,
            r#"SELECT "documents"."id", "documents"."tenant_id", "documents"."title" FROM "documents" WHERE "documents"."title" = 'roadmap' AND "documents"."tenant_id" = 42"#
        );
    }

    #[test]
    fn scopes_bulk_updates() {
        let query = documents::Entity::update_many()
            .col_expr(documents::Column::Title, Expr::value("released"))
            .in_tenant(42)
            .build(DbBackend::Postgres)
            .to_string();

        assert_eq!(
            query,
            r#"UPDATE "documents" SET "title" = 'released' WHERE "documents"."tenant_id" = 42"#
        );
    }

    #[test]
    fn scopes_bulk_deletes() {
        let query = documents::Entity::delete_many()
            .in_tenant(42)
            .build(DbBackend::Postgres)
            .to_string();

        assert_eq!(
            query,
            r#"DELETE FROM "documents" WHERE "documents"."tenant_id" = 42"#
        );
    }

    #[test]
    fn sets_tenant_on_new_active_models() {
        let document = documents::ActiveModel {
            title: Set("roadmap".to_owned()),
            ..Default::default()
        }
        .set_tenant(42)
        .unwrap();

        assert_eq!(document.tenant_id, Set(42));
    }

    #[test]
    fn accepts_an_active_model_already_assigned_to_the_same_tenant() {
        let document = documents::ActiveModel {
            tenant_id: Set(42),
            ..Default::default()
        }
        .set_tenant(42)
        .unwrap();

        assert_eq!(document.tenant_id, Set(42));
    }

    #[test]
    fn accepts_an_existing_model_owned_by_the_same_tenant() {
        let document = documents::ActiveModel {
            tenant_id: sea_orm::ActiveValue::Unchanged(42),
            ..Default::default()
        }
        .set_tenant(42)
        .unwrap();

        assert_eq!(document.tenant_id, sea_orm::ActiveValue::Unchanged(42));
    }

    #[test]
    fn rejects_a_different_tenant_on_new_active_models() {
        let result = documents::ActiveModel {
            tenant_id: Set(7),
            ..Default::default()
        }
        .set_tenant(42);

        assert!(matches!(result, Err(ModelError::TenantMismatch)));
    }

    #[test]
    fn rejects_changing_the_tenant_on_existing_models() {
        let result = documents::ActiveModel {
            tenant_id: sea_orm::ActiveValue::Unchanged(7),
            ..Default::default()
        }
        .set_tenant(42);

        assert!(matches!(result, Err(ModelError::TenantMismatch)));
    }

    #[test]
    fn reports_a_misconfigured_tenant_id_type() {
        let result =
            <malformed_documents::ActiveModel as Default>::default().set_tenant("acme".to_owned());

        assert!(matches!(result, Err(ModelError::DbErr(_))));
    }
}
