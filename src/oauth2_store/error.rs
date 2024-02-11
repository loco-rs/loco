use oauth2::{
    basic::BasicErrorResponseType, url::ParseError, RequestTokenError, StandardErrorResponse,
};

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
