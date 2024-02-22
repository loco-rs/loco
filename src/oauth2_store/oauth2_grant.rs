use std::sync::Arc;

use tokio::sync::Mutex;

use crate::oauth2_store::grants::authorization_code::AuthorizationCodeGrantTrait;

#[derive(Clone)]
pub enum OAuth2ClientGrantEnum {
    AuthorizationCode(Arc<Mutex<dyn AuthorizationCodeGrantTrait>>),
    ClientCredentials,
    DeviceCode,
    Implicit,
    ResourceOwnerPasswordCredentials,
}
