use oauth2::{
    basic::BasicErrorResponseType, url::ParseError, DeviceCodeErrorResponseType,
    RevocationErrorResponseType,
};

#[derive(thiserror::Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum OAuth2StoreError {
    #[error(transparent)]
    BasicError(#[from] BasicErrorResponseType),
    #[error(transparent)]
    DeviceCodeError(#[from] DeviceCodeErrorResponseType),
    #[error(transparent)]
    RevocationError(#[from] RevocationErrorResponseType),
}

#[allow(clippy::module_name_repetitions)]
#[derive(thiserror::Error, Debug)]
pub enum OAuth2ClientError {
    #[error(transparent)]
    ClientError(#[from] ParseError),
}

pub type OAuth2ClientResult<T> = std::result::Result<T, OAuth2ClientError>;

pub type OAuth2StoreResult<T> = std::result::Result<T, OAuth2StoreError>;
