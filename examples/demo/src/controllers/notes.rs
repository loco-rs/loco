#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use axum::extract::Query;
use loco_rs::{controller::bad_request, model::ModelError, prelude::*};
use sea_orm::Condition;
use serde::{Deserialize, Serialize};

use crate::{
    models::_entities::notes::{ActiveModel, Column, Entity, Model},
    views::notes::PaginationResponse,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Params {
    pub title: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListQueryParams {
    pub title: Option<String>,
    pub content: Option<String>,
    #[serde(flatten)]
    pub pagination: query::PaginationQuery,
}

impl Params {
    fn update(&self, item: &mut ActiveModel) {
        item.title = Set(self.title.clone());
        item.content = Set(self.content.clone());
    }
}

async fn load_item(ctx: &AppContext, id: i32) -> Result<Model> {
    let item = Entity::find_by_id(id).one(&ctx.db).await?;
    item.ok_or_else(|| Error::NotFound)
}

pub async fn list(
    State(ctx): State<AppContext>,
    Query(params): Query<ListQueryParams>,
) -> Result<Response> {
    let pagination_query = query::PaginationQuery {
        page_size: params.pagination.page_size,
        page: params.pagination.page,
    };

    let paginated_notes = query::paginate(
        &ctx.db,
        Entity::find(),
        Some(query::with(params.into_query()).build()),
        &pagination_query,
    )
    .await?;

    /*
    if let Some(settings) = &ctx.config.settings {
        let settings = common::settings::Settings::from_json(settings)?;
        println!("allow list: {:?}", settings.allow_list);
    }*/

    format::render()
        .cookies(&[
            cookie::Cookie::new("foo", "bar"),
            cookie::Cookie::new("baz", "qux"),
        ])?
        .etag("foobar")?
        .json(PaginationResponse::response(
            paginated_notes,
            &pagination_query,
        ))
}

pub async fn add(State(ctx): State<AppContext>, Json(params): Json<Params>) -> Result<Response> {
    let mut item = ActiveModel {
        ..Default::default()
    };
    params.update(&mut item);
    let item = item.insert(&ctx.db).await?;
    format::json(item)
}

pub async fn update(
    Path(id): Path<i32>,
    State(ctx): State<AppContext>,
    Json(params): Json<Params>,
) -> Result<Response> {
    let item = load_item(&ctx, id).await?;
    let mut item = item.into_active_model();
    params.update(&mut item);
    let item = item.update(&ctx.db).await?;
    format::json(item)
}

pub async fn remove(Path(id): Path<i32>, State(ctx): State<AppContext>) -> Result<Response> {
    load_item(&ctx, id).await?.delete(&ctx.db).await?;
    format::empty()
}

pub async fn get_one(
    Format(respond_to): Format,
    Path(id): Path<i32>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    // having `load_item` is useful because inside the function you can call and use
    // '?' to bubble up errors, then, in here, we centralize handling of errors.
    // if you want to freely use code statements with no wrapping function, you can
    // use the experimental `try` feature in Rust where you can do:
    // ```
    // let res = try {
    //     ...
    //     ...
    // }
    //
    // match res { ..}
    // ```
    let res = load_item(&ctx, id).await;

    match res {
        // we're good, let's render the item based on content type
        Ok(item) => match respond_to {
            RespondTo::Html => format::html(&format!("<html><body>{:?}</body></html>", item.title)),
            _ => format::json(item),
        },
        // we have an opinion how to render out validation errors, only in HTML content
        Err(Error::Model(ModelError::ModelValidation { errors })) => match respond_to {
            RespondTo::Html => {
                format::html(&format!("<html><body>errors: {errors:?}</body></html>"))
            }
            _ => bad_request("opaque message: cannot respond!"),
        },
        // we have no clue what this is, let the framework render default errors
        Err(err) => Err(err),
    }
}

impl ListQueryParams {
    #[must_use]
    pub fn into_query(&self) -> Condition {
        let mut condition = query::condition();

        if let Some(title) = &self.title {
            condition = condition.like(Column::Title, title);
        }
        if let Some(content) = &self.content {
            condition = condition.like(Column::Content, content);
        }
        condition.build()
    }
}

pub fn routes() -> Routes<AppContext> {
    Routes::new()
        .prefix("notes")
        .add("/", get(list))
        .add("/", post(add))
        .add("/:id", get(get_one))
        .add("/:id", delete(remove))
        .add("/:id", post(update))
}
