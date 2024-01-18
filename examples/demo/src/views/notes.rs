use loco_rs::controller::views::pagination::PaginationResponseTrait;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

use crate::models::_entities::notes;

#[derive(Debug, Deserialize, Serialize)]
pub struct ListResponse {
    title: Option<String>,
    content: Option<String>,
}

impl PaginationResponseTrait for ListResponse {
    type Model = crate::models::_entities::notes::Entity;
    type ResponseType = Self;

    fn list(models: Vec<<Self::Model as EntityTrait>::Model>) -> Vec<Self::ResponseType> {
        models.into_iter().map(|a| Self::new(&a)).collect()
    }
}

impl ListResponse {
    #[must_use]
    pub fn new(note: &notes::Model) -> Self {
        Self {
            title: note.title.clone(),
            content: note.content.clone(),
        }
    }
}
