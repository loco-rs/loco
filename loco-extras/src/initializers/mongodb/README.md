# This Initializer adds support for connection to a MongoDB database

There is extra functionality that loco supports through the `loco_extras` crate. Each extra can be pulled in optionally and is intgerated into your loco app by adding them as intializers.

This initializer adds support for using a MongoDB database. Choosing to use Mongo would mean sacrificing a lot of the features that loco provides out of the box, such as user authentication, but it can still be used quite effectively as loco will help with a lot of the boilerplate code.

This initializer is recommended to be used with the base starter that does not come with a database connection (as those all use SQL), but it can be used with any other starter as well.

## Steps

To add this initializer to your project, follow these steps:

### Add Dependencies

Add the `mongodb` crate and mongodb initializer to your loco project.

```toml
# Cargo.toml
[dependencies]
loco-extras = { version = "*", features = ["mongodb"] }
mongodb = { version = "2.8.0"}
```

### Add to the Config

Add a mongodb connection section to you config.yaml file.

```yaml
# config/[development/test/production...].yaml
initializers:
  mongodb:
    uri:  {{ get_env(name="MONGODB_URI", default="mongodb://localhost:27017/") }}
    db_name: "db_name"
    client_options:
      connectTimeout:
        secs: 2
        nanos: 0
      serverSelectionTimeout:
        secs: 2
        nanos: 0
```

Although untested, the `client_options` should be able to take any options that the mongodb driver supports. The options are passed as a `serde_json::Value`, so however that type is serialized/deserialized should work here. Example above shows how `Duration` is serialized.


### Add the Initializer

Add the initializer to your `src/app.rs` file.

```rust
// src/app.rs
pub struct App;
#[async_trait]
impl Hooks for App {
    // Other code...
    async fn initializers(ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        let mut initializers: Vec<Box<dyn Initializer>> = vec![
            Box::new(loco_extras::initializers::mongodb::MongoDbInitializer),
        ];

        Ok(initializers)
    }
    // Other code...
}
```

### Using the Connection

Now you can use the connection in your handler code.

```rust
// src/controllers/mongo.rs
use axum::Extension;
use loco_rs::prelude::*;
use serde_json::Value;
use mongodb::{bson::doc, Database};

pub async fn fetch(Extension(mongodb): Extension<Database>) -> Result<Response> {
    let user: Option<Value> = mongodb.collection("users").find_one(doc!{}, None).await.map_err(|_| Error::NotFound)?;
    format::json(user.ok_or_else(|| Error::NotFound)?)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("mongo")
        .add("/", get(fetch))
}
```

If you are adding a new file, don't forget to add it to the `src/controllers/mod.rs` file.

### Adding to the Router

If you created a new controller, you need to register the routes in your `src/app.rs` file.

```rust
// src/app.rs

fn routes(ctx: &AppContext) -> AppRoutes {
    AppRoutes::with_default_routes()
        // Other routes...
        .add_route(controllers::mongodb::routes())
}
```

Now you can run the server with the config information set OR set the `MONGODB_URI` environment variable.
