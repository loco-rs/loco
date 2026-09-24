# Recipe: models and migrations

Loco uses Sea-ORM. The schema is owned by migrations; entities are **generated
from the database**, never written by hand.

## Add a model

```sh
cargo loco generate model posts title:string! content:text published:bool user:references
cargo loco db migrate
```

That writes the migration, runs it, regenerates `src/models/_entities/posts.rs`,
and creates `src/models/posts.rs` for your code.

### Column DSL

`field:spec`. Suffixes are orthogonal flags:

| Suffix | Meaning |
|---|---|
| *(none)* | nullable |
| `!` | `NOT NULL` |
| `^` | unique **and** required |

Base types: `string` `text` `int` `small_int` `big_int` `unsigned`
`small_unsigned` `big_unsigned` `float` `double` `decimal` `money` `bool`
`date` `date_time` `tstz` `time` `uuid` `json` `jsonb` `blob` `binary_len:N`
`var_binary:N` `decimal_len:P:S` `enum:a,b,c` `array:inner`.

Foreign keys use `references`, and **invert the convention on purpose**:

| Spec | Meaning |
|---|---|
| `user:references` | FK to `users`, `NOT NULL` |
| `user:references?` | FK to `users`, nullable |
| `author:references:users` | FK to `users` via a custom column name |

```sh
cargo loco generate model comments content:text! post:references user:references?
```

## The two files, and which one you touch

| File | Owner | Rule |
|---|---|---|
| `src/models/_entities/posts.rs` | the generator | **never hand-edit** — regenerated from the schema, your changes are destroyed |
| `src/models/posts.rs` | you | all domain logic; re-exports from `_entities` |

`src/models/posts.rs` starts as:

```rust
use loco_rs::prelude::*;
pub use super::_entities::posts::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for ActiveModel {}
```

## Where your code goes

**Finders and creation → `impl Model`.** Rails synthesizes `find_by_email`;
Rust makes you write it, but the doctrine is the same — a named finder on the
model, not a raw query in a controller.

```rust
impl Model {
    /// # Errors
    /// Returns `ModelError::EntityNotFound` if no such post exists.
    pub async fn find_by_slug(db: &DatabaseConnection, slug: &str) -> ModelResult<Self> {
        posts::Entity::find()
            .filter(model::query::condition().eq(posts::Column::Slug, slug).build())
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)
    }
}
```

**State transitions → `impl ActiveModel`,** consuming `self`, returning the
persisted `Model`:

```rust
impl ActiveModel {
    pub async fn publish(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.published = ActiveValue::Set(true);
        self.published_at = ActiveValue::Set(Some(Local::now().into()));
        Ok(self.update(db).await?)
    }
}
```

Call it as `post.into_active_model().publish(&ctx.db).await?`.

**Multi-statement invariants need a transaction.** A check-then-insert without
one is a race, not a validation:

```rust
impl Model {
    pub async fn create_unique(db: &DatabaseConnection, slug: &str) -> ModelResult<Self> {
        let txn = db.begin().await?;
        if posts::Entity::find()
            .filter(model::query::condition().eq(posts::Column::Slug, slug).build())
            .one(&txn)
            .await?
            .is_some()
        {
            return Err(ModelError::EntityAlreadyExists {});
        }
        let post = posts::ActiveModel {
            slug: ActiveValue::set(slug.to_string()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;
        Ok(post)
    }
}
```

## Validation

`Validatable` declares the rules; `ActiveModelBehavior::before_save` enforces
them and fills derived fields. Do not duplicate a rule inline in a handler.

Three pieces, and the first is the one that gets left out: a plain struct
carrying the `validator` crate's attributes. The `ActiveModel` cannot carry them
itself — its fields are `ActiveValue<T>`, not `T` — which is the whole reason
this indirection exists.

```rust
use loco_rs::prelude::*;          // brings `Validate` and `Validatable`
use serde::Deserialize;

#[derive(Debug, Validate, Deserialize)]
pub struct Validator {
    #[validate(length(min = 2, message = "Name must be at least 2 characters long."))]
    pub name: String,
    #[validate(email(message = "invalid email"))]
    pub email: String,
}

impl Validatable for ActiveModel {
    fn validator(&self) -> Box<dyn Validate> {
        Box::new(Validator {
            name: self.name.as_ref().to_owned(),
            email: self.email.as_ref().to_owned(),
        })
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        self.validate()?;
        if insert {
            let mut this = self;
            this.pid = ActiveValue::Set(Uuid::new_v4());
            return Ok(this);
        }
        Ok(self)
    }
}
```

A failed `validate()` becomes `ModelError::Validation`, which the framework
already renders as a **400** carrying the per-field messages. There is nothing
to catch in the handler — see `endpoint.md`. The type itself is
`validation::ModelValidationErrors`, not a bare prelude name.

`src/models/users.rs` in the starter app is a working instance of all three
pieces.

## Changing an existing schema

Never edit a migration that has already run. Generate a new one:

```sh
cargo loco generate migration AddViewsToPosts views:int!
cargo loco db migrate
cargo loco db entities        # regenerate _entities from the new schema
```

## Commands

```sh
cargo loco db migrate      # apply pending migrations
cargo loco db entities     # regenerate _entities/ from the live schema
cargo loco db reset        # drop, recreate, migrate (destructive)
cargo loco db seed         # load seeds
cargo loco db status
```
