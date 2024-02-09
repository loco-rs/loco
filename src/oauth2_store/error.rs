use oauth2::{
    basic::{BasicErrorResponse, BasicErrorResponseType},
    reqwest::Error,
    url::ParseError,
    RequestTokenError, StandardErrorResponse,
};

#[derive(thiserror::Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum OAuth2StoreError {}

#[allow(clippy::module_name_repetitions)]
#[derive(thiserror::Error, Debug)]
pub enum OAuth2ClientError {
    #[error(transparent)]
    UrlError(#[from] ParseError),
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
    #[error(transparent)]
    BasicTokenError(#[from] BasicTokenError),
    #[error("CSRF token error")]
    CsrfTokenError,
    #[error("Profile error")]
    ProfileError(reqwest::Error),
}
type BasicTokenError = RequestTokenError<
    oauth2::reqwest::Error<reqwest::Error>,
    StandardErrorResponse<BasicErrorResponseType>,
>;

pub type OAuth2ClientResult<T> = std::result::Result<T, OAuth2ClientError>;

pub type OAuth2StoreResult<T> = std::result::Result<T, OAuth2StoreError>;
