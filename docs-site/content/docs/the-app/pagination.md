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
flair =[]
+++

In many scenarios, when querying data and returning responses to users, pagination is crucial. In `Loco`, we provide a straightforward method to paginate your data and maintain a consistent pagination response schema for your API responses.

How to Use Pagination
```rust
use loco_rs::prelude::*;

let pagination_query = model::query::PaginationQuery {
    page_size: 100,
    page: 1,
};

let condition = model::query::dsl::condition().contains(notes::Column::Title, "loco");
let paginated_notes = model::query::exec::paginate(
    &ctx.db,
    notes::Entity::find(),
    Some(condition.build()),
    &pagination_query,
)
.await?;
```

- Start by defining the entity you want to retrieve.
- Create your query condition (in this case, filtering rows that contain "loco" in the title column).
- Define the pagination parameters.
- Call the paginate function.


###### Setting Up Pagination View
After creating getting the `paginated_notes` in the previous example, you can choose which fileds from the model you want to return and keep the same pagination response in all your different data responses.

Define the data you're returning to the user in Loco views. If you're not familiar with views, refer to the [documentation]((@/docs/the-app/views.md)) for more context.


Create a notes view file in `src/view/notes` with the following code:

```rust
use loco_rs::{
    controller::views::pagination::{Pager, PagerMeta},
    prelude::model::query::PaginatedResponse,
};
use serde::{Deserialize, Serialize};

use crate::models::_entities::notes;

#[derive(Debug, Deserialize, Serialize)]
pub struct ListResponse {
    id: i32,
    title: Option<String>,
    content: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PaginationResponse {}

impl From<notes::Model> for ListResponse {
    fn from(note: notes::Model) -> Self {
        Self {
            id: note.id.clone(),
            title: note.title.clone(),
            content: note.content,
        }
    }
}

impl PaginationResponse {
    #[must_use]
    pub fn response(data: PaginatedResponse<notes::Model>) -> Pager<Vec<ListResponse>> {
        Pager {
            results: data
                .rows
                .into_iter()
                .map(ListResponse::from)
                .collect::<Vec<ListResponse>>(),
            info: PagerMeta {
                page: data.info.page,
                page_size: data.info.page_size,
                total_pages: data.info.total_pages,
            },
        }
    }
}
