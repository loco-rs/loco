use std::{borrow::Cow, collections::BTreeMap};

use crate::oauth2_storage::oauth2_grant::OAuth2ClientGrantEnum;

// #[cfg(feature = "oauth2")]
mod error;
mod grants;
pub mod oauth2_grant;

#[derive(Clone)]
pub struct OAuth2ClientStore {
    pub clients: BTreeMap<String, OAuth2ClientGrantEnum>,
}

impl OAuth2ClientStore {
    /// Create a new instance of `OAuth2ClientStore`.
    #[must_use]
    pub fn new(clients: BTreeMap<Cow<&str>, OAuth2ClientGrantEnum>) -> Self {
        Self {
            clients: BTreeMap::new(),
        }
    }
}
