use std::sync::Arc;

use crate::oauth2_storage::grants::authorization_code::AuthorizationCodeGrantTrait;

#[derive(Clone)]
pub enum OAuth2ClientGrantEnum {
    AuthorizationCode(Arc<Box<dyn AuthorizationCodeGrantTrait>>),
    ClientCredentials,
    DeviceCode,
    Implicit,
    ResourceOwnerPasswordCredentials,
}
