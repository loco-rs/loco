use sea_orm::prelude::{DateTimeWithTimeZone, Decimal};
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
#[serde(rename_all = "snake_case")]
pub enum PostStatus {
    Draft,
    Published,
    Archived,
}

impl From<String> for PostStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "published" => Self::Published,
            "archived" => Self::Archived,
            _ => Self::Draft,
        }
    }
}

impl PostStatus {
    /// Returns the `snake_case` string used to persist this status in the
    /// `posts.status` column (kept in sync with the `serde` representation).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Published => "published",
            Self::Archived => "archived",
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct PostDto {
    #[ts(type = "number")]
    pub id: i64,
    pub title: String,
    pub content: String,
    pub status: PostStatus,
    #[ts(type = "string")]
    pub price: Decimal,
    #[ts(type = "string | null")]
    pub published_at: Option<DateTimeWithTimeZone>,
    #[ts(type = "string")]
    pub created_at: DateTimeWithTimeZone,
}

impl From<crate::models::_entities::posts::Model> for PostDto {
    fn from(m: crate::models::_entities::posts::Model) -> Self {
        Self {
            id: m.id,
            title: m.title,
            content: m.content,
            status: PostStatus::from(m.status),
            price: m.price,
            published_at: m.published_at,
            created_at: m.created_at,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct CreatePost {
    pub title: String,
    pub content: String,
    pub status: PostStatus,
    #[ts(type = "string")]
    pub price: Decimal,
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct UpdatePost {
    pub title: String,
    pub content: String,
    pub status: PostStatus,
    #[ts(type = "string")]
    pub price: Decimal,
}
