use std::collections::BTreeMap;

use crate::oauth2_store::oauth2_grant::OAuth2ClientGrantEnum;

// #[cfg(feature = "oauth2")]
pub mod error;
pub mod grants;
pub mod oauth2_grant;

#[derive(Clone)]
pub struct OAuth2ClientStore {
    pub clients: BTreeMap<String, OAuth2ClientGrantEnum>,
}

impl OAuth2ClientStore {
    /// Create a new instance of `OAuth2ClientStore`.
    #[must_use]
    pub fn new(clients: BTreeMap<String, OAuth2ClientGrantEnum>) -> Self {
        Self { clients }
    }

    /// Get a client by its id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&OAuth2ClientGrantEnum> {
        self.clients.get(id)
    }
}
