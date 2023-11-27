// TODO(review): base components must be re-exported
use loco_rs::{
    auth,
    model::{ModelError, ModelResult},
    validation,
    validator::Validate,
};
use sea_orm::{
    entity::prelude::*, ActiveValue, DatabaseConnection, DbErr, ModelTrait, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use super::_entities::users::{self, ActiveModel, Entity, Model};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginParams {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterParams {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Validate, Deserialize)]
pub struct ModelValidator {
    #[validate(length(min = 2, message = "Name must be at least 2 characters long."))]
    pub name: String,
    #[validate(custom = "validation::is_valid_email")]
    pub email: String,
}

impl From<&ActiveModel> for ModelValidator {
    fn from(value: &ActiveModel) -> Self {
        Self {
            name: value.name.as_ref().to_string(),
            email: value.email.as_ref().to_string(),
        }
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for super::_entities::users::ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        {
            self.validate()?;
            if insert {
                let mut this = self;
                this.pid = ActiveValue::Set(Uuid::new_v4());
                Ok(this)
            } else {
                Ok(self)
            }
        }
    }
}

impl super::_entities::users::Model {
    /// .
    ///
    /// # Errors
    ///
    /// .
    pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// .
    ///
    /// # Errors
    ///
    /// .
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        let parse_uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Message(e.to_string()))?;
        let user = users::Entity::find()
            .filter(users::Column::Pid.eq(parse_uuid))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// .
    ///
    /// # Errors
    ///
    /// .
    pub fn verify_password(&self, password: &str) -> ModelResult<bool> {
        Ok(auth::verify_password(password, &self.password)?)
    }

    /// .
    ///
    /// # Errors
    ///
    /// .
    pub async fn create_with_password(
        db: &DatabaseConnection,
        params: &RegisterParams,
    ) -> ModelResult<Self> {
        let txn = db.begin().await?;

        if users::Entity::find()
            .filter(users::Column::Email.eq(&params.email))
            .one(&txn)
            .await?
            .is_some()
        {
            return Err(ModelError::EntityExists {});
        }

        let password_hash = auth::hash_password(&params.password)?;
        let user = users::ActiveModel {
            //TODO(review): there might be a 'trick' to moves between params and partial
            // ActiveValue values to save this rhs-lhs coding
            email: ActiveValue::set(params.email.to_string()),
            password: ActiveValue::set(password_hash),
            name: ActiveValue::set(params.name.to_string()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(user)
    }

    /// .
    ///
    /// # Errors
    ///
    /// .
    pub fn generate_jwt(&self, secret: &str, expiration: &u64) -> ModelResult<String> {
        Ok(auth::JWT::new(secret).generate_token(expiration, self.pid.to_string())?)
    }
}

impl super::_entities::users::ActiveModel {
    /// .
    ///
    /// # Errors
    ///
    /// .
    pub fn validate(&self) -> Result<(), DbErr> {
        let validator: ModelValidator = self.into();
        validator.validate().map_err(validation::into_db_error)
    }
}
