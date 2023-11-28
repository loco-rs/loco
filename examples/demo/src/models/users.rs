// TODO(review): base components must be re-exported
use chrono::offset::Local;
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
    /// Returns list of user notes
    ///
    /// # Errors
    ///
    /// Return an error when could not complete the DB query
    pub async fn notes(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Vec<super::_entities::notes::Model>, DbErr> {
        self.find_related(super::_entities::prelude::Notes)
            .all(db)
            .await
    }

    /// Finding user by email
    ///
    /// # Errors
    ///
    /// When could not find the user or DB query error
    pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Finding user by verification token
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_verification_token(
        db: &DatabaseConnection,
        token: &str,
    ) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(users::Column::EmailVerificationToken.eq(token))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Finding user by reset token
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_reset_token(db: &DatabaseConnection, token: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(users::Column::ResetToken.eq(token))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Finding user by pid
    ///
    /// # Errors
    ///
    /// When could not find user  or DB query error
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        let parse_uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Message(e.to_string()))?;
        let user = users::Entity::find()
            .filter(users::Column::Pid.eq(parse_uuid))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Check if the given plain password is equal to hashed password that store
    /// in DB
    ///
    /// # Errors
    ///
    /// when could not verify password
    pub fn verify_password(&self, password: &str) -> ModelResult<bool> {
        Ok(auth::verify_password(password, &self.password)?)
    }

    /// Creates a user with password
    ///
    /// # Errors
    ///
    /// When could not save the user into the DB
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

    /// Creates a JWT
    ///
    /// # Errors
    ///
    /// when could not convert user claims to jwt token
    pub fn generate_jwt(&self, secret: &str, expiration: &u64) -> ModelResult<String> {
        Ok(auth::JWT::new(secret).generate_token(expiration, self.pid.to_string())?)
    }
}

impl super::_entities::users::ActiveModel {
    /// Validate user schema
    ///
    /// # Errors
    ///
    /// when the active model is not valid
    pub fn validate(&self) -> Result<(), DbErr> {
        let validator: ModelValidator = self.into();
        validator.validate().map_err(validation::into_db_error)
    }

    /// Save verification token
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_email_verification_sent(
        mut self,
        db: &DatabaseConnection,
    ) -> ModelResult<Model> {
        self.email_verification_sent_at = ActiveValue::set(Some(Local::now().naive_local()));
        self.email_verification_token = ActiveValue::Set(Some(Uuid::new_v4().to_string()));
        Ok(self.update(db).await?)
    }

    /// Save reset password token
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_forgot_password_sent(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.reset_sent_at = ActiveValue::set(Some(Local::now().naive_local()));
        self.reset_token = ActiveValue::Set(Some(Uuid::new_v4().to_string()));
        Ok(self.update(db).await?)
    }

    /// Save verify time when user verify his email
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn verified(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.email_verified_at = ActiveValue::set(Some(Local::now().naive_local()));
        Ok(self.update(db).await?)
    }

    /// Reset current password with new password
    ///
    /// # Errors
    ///
    /// when has DB query error or could not hashed the given password
    pub async fn reset_password(
        mut self,
        db: &DatabaseConnection,
        password: &str,
    ) -> ModelResult<Model> {
        self.password = ActiveValue::set(auth::hash_password(password)?);
        Ok(self.update(db).await?)
    }
}
