use chrono::{Duration, Local};
use loco_rs::{
    model::{ModelError, ModelResult},
    oauth2_store::{basic::BasicTokenResponse, TokenResponse},
};
use sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait};

pub use super::_entities::sessions::{self, ActiveModel, Model};
use crate::models::users;
#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    // extend activemodel below (keep comment for generators)
}

impl super::_entities::sessions::Model {
    pub async fn create_session(
        db: &DatabaseConnection,
        token: &BasicTokenResponse,
        user: &users::Model,
    ) -> ModelResult<Self> {
        let txn = db.begin().await?;
        // Set the cookie
        let secs: i64 = token.expires_in().unwrap().as_secs().try_into().unwrap();
        let session = sessions::ActiveModel {
            session_id: ActiveValue::set(token.access_token().secret().clone()),
            // Set the cookie to expire when the token expires
            expires_at: ActiveValue::set(Local::now().naive_local() + Duration::seconds(secs)),
            user_id: ActiveValue::set(user.id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;
        Ok(session)
    }

    pub async fn is_expired(db: &DatabaseConnection, session_id: &str) -> ModelResult<bool> {
        let session = sessions::Entity::find()
            .filter(sessions::Column::SessionId.eq(session_id))
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;
        Ok(session.expires_at < Local::now().naive_local())
    }
}
