+++
title = "Pagination"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 20
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++

In many scenarios, when querying data and returning responses to users, pagination is crucial. In `Loco`, we provide a straightforward method to paginate your data and maintain a consistent pagination response schema for your API responses.

How to Use Pagination


```rust
use sea_orm::Condition;
use loco_rs::concern::pagination;

let notes_query = Entity::find();
let mut condition = Condition::all().add(Column::Title.like("%loco%"));
let pagination = pagination::PaginationFilter {
    page_size: 10,
    page: 5
};
let results = pagination::paginate(
    &ctx.db,
    notes_query,
    Some(condition),
    &params.pagination,
)
.await?;
```

- Start by defining the entity you want to retrieve.
- Create your query condition (in this case, filtering rows that contain "loco" in the title column).
- Define the pagination parameters.
- Call the paginate function.


### Using Pagination in Controller
In most cases, you'll want to implement pagination in your REST API responses. Let's create a notes endpoint as an example.


###### Setting Up Pagination View
Define the data you're returning to the user in Loco views. If you're not familiar with views, refer to the [documentation]((@/docs/the-app/views.md)) for more context.


Create a notes view file in `src/view/notes` with the following code:

```rust
use crate::models::_entities::notes;
use loco_rs::controller::views::pagination::PaginationResponseTrait;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

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

```

- `ListResponse` defines the fields returned to the user.
- Implement `PaginationResponseTrait` for `ListResponse` and specify the related model, creating a list function to convert models into view responses.

###### Implementing Pagination View
Now, create a `list` function endpoint under the notes endpoint and define `ListQueryParams` as the query parameters:

```rust
use loco_rs::concern::pagination;
use axum::extract::Query;
use crate::{
    models::_entities::notes::Entity,
};


#[derive(Debug, Deserialize)]
pub struct ListQueryParams {
    pub title: Option<String>,
    pub content: Option<String>,
    #[serde(flatten)]
    pub pagination: pagination::PaginationFilter,
}


pub async fn list(
    State(ctx): State<AppContext>,
    Query(params): Query<ListQueryParams>,
) -> Result<Json<Pager<Vec<ListResponse>>>> {
    let notes_query = Entity::find();
    let mut condition = Condition::all();// .add(Column::Title.like("%loco%"));

    let notes: Pager<Vec<ListResponse>> =
        pagination::view::<ListResponse, crate::models::_entities::notes::Entity>(
            &ctx.db,
            notes_query,
            Some(params.into_query()),
            &params.pagination,
        )
        .await?;

    format::json(notes)
}
```

- `ListQueryParams` defines the allowed query parameters for your REST API.
- The `title` and `content` parameters filter rows in the database.
- The `pagination` parameter specifies the pagination parameters for the query.


This outlines your controller implementation. Define the parameters you want to return in your REST API view, then specify the query parameters, and you'll have pagination for any entity in your application.
