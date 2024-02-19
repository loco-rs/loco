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
    pub async fn upsert_with_oauth(
        db: &DatabaseConnection,
        token: &BasicTokenResponse,
        user: &users::Model,
    ) -> ModelResult<Self> {
        let txn = db.begin().await?;
        let session_id = token.access_token().secret().clone();
        let session = match sessions::Entity::find()
            .filter(sessions::Column::UserId.eq(user.id))
            .one(&txn)
            .await?
        {
            Some(session) => {
                // Update the session
                let mut session: sessions::ActiveModel = session.into();
                session.session_id = ActiveValue::set(session_id);
                session.expires_at =
                    ActiveValue::set(Local::now().naive_local() + token.expires_in().unwrap());
                session.updated_at = ActiveValue::set(Local::now().naive_local());
                session.update(&txn).await?
            }
            None => {
                // Create the session
                sessions::ActiveModel {
                    session_id: ActiveValue::set(session_id),
                    expires_at: ActiveValue::set(
                        Local::now().naive_local() + token.expires_in().unwrap(),
                    ),
                    user_id: ActiveValue::set(user.id),
                    ..Default::default()
                }
                .insert(&txn)
                .await?
            }
        };
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
