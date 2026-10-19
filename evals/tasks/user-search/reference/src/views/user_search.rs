// `Pager`/`PagerMeta` come from the prelude. This file used to reach for
// `loco_rs::controller::views::pagination::{..}` by hand, which is what an
// agent cannot guess — and the eval caught one hand-rolling the envelope
// instead. The deep import staying gone is the regression test.
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::_entities::users;

/// The public projection of a user.
///
/// The `users` entity carries `password` (a hash) and `api_key`. Serialising
/// the entity directly would put both on the wire — and would compile, lint,
/// and test clean. Only these three fields are ever public.
#[derive(Debug, Deserialize, Serialize)]
pub struct UserSummary {
    pub pid: String,
    pub name: String,
    pub email: String,
}

impl From<&users::Model> for UserSummary {
    fn from(user: &users::Model) -> Self {
        Self {
            pid: user.pid.to_string(),
            name: user.name.clone(),
            email: user.email.clone(),
        }
    }
}

impl UserSummary {
    /// Wraps a page of users in the framework's pager envelope.
    #[must_use]
    pub fn page(users: &[users::Model], meta: PagerMeta) -> Pager<Vec<Self>> {
        Pager::new(users.iter().map(Self::from).collect(), meta)
    }
}
