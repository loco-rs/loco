pub use super::_entities::roles::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::TenantEntity;
use sea_orm::entity::prelude::*;
pub type Roles = Entity;

impl TenantEntity for Entity {
    type TenantId = i64;

    fn tenant_column() -> Column {
        Column::TenantId
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !insert && self.updated_at.is_unchanged() {
            let mut this = self;
            this.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now().into());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

// implement your read-oriented logic here
impl Model {}

// implement your write-oriented logic here
impl ActiveModel {}

// implement your custom finders, selectors oriented logic here
impl Entity {}
