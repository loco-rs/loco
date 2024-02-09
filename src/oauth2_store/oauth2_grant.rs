use crate::oauth2_storage::grants::authorization_code::AuthorizationCodeGrantTrait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub enum OAuth2ClientGrantEnum {
    AuthorizationCode(Arc<Mutex<dyn AuthorizationCodeGrantTrait>>),
    ClientCredentials,
    DeviceCode,
    Implicit,
    ResourceOwnerPasswordCredentials,
}
