use std::{borrow::Cow, collections::BTreeMap};

use crate::oauth2::oauth2::OAuth2Client;

// #[cfg(feature = "oauth2")]
mod error;
pub mod oauth2;

#[derive(Clone)]
pub struct OAuth2ClientStore {
    pub clients: BTreeMap<String, OAuth2Client>,
}

impl OAuth2ClientStore {
    /// Create a new instance of `OAuth2ClientStore`.
    #[must_use]
    pub fn new(clients: BTreeMap<Cow<&str>, OAuth2Client>) -> Self {
        Self {
            clients: BTreeMap::new(),
        }
    }
}
