---
title: Validate requests
description: Validate JSON, form, and query-string payloads with Loco's validating extractors, and return structured or simplified errors.
sidebar:
  order: 11
---

**Goal:** reject malformed input before your handler logic runs, and return either a simple `400 Bad Request` or a structured, field-by-field JSON error body.

This assumes a working controller — see [Add a controller](/docs/how-to/add-controller) if you need one first. All six extractors live in `loco_rs::controller::extractor::validate` (`JsonValidate`/`JsonValidateWithMessage` are re-exported from `loco_rs::prelude`).

## 1. Pick an extractor

| Extractor | Content type | Structured JSON errors? |
|---|---|---|
| `JsonValidate<T>` | `application/json` | No — plain `400 Bad Request` |
| `JsonValidateWithMessage<T>` | `application/json` | Yes |
| `FormValidate<T>` | `application/x-www-form-urlencoded` | No — plain `400 Bad Request` |
| `FormValidateWithMessage<T>` | `application/x-www-form-urlencoded` | Yes |
| `QueryValidate<T>` | any (reads the query string) | No — plain `400 Bad Request` |
| `QueryValidateWithMessage<T>` | any (reads the query string) | Yes |

Each is a `FromRequest` newtype: `JsonValidate<T>(pub T)`, etc. — pattern-match to get at the inner, already-validated `T`.

## 2. Define the validated type

Derive `validator::Validate`:

```rust
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateNote {
    #[validate(length(min = 3, message = "title must be at least 3 characters"))]
    pub title: String,
    #[validate(email)]
    pub email: String,
}
```

`validator::Validate` is automatically adapted to Loco's own `ValidatorTrait` — you don't implement anything extra to use it with the extractors above.

## 3. Use the extractor in a handler

```rust
use loco_rs::prelude::*;

#[debug_handler]
pub async fn create(
    State(_ctx): State<AppContext>,
    JsonValidate(params): JsonValidate<CreateNote>,
) -> Result<Response> {
    // `params` is guaranteed valid here
    format::json(params)
}
```

Swap the extractor type to change source/behavior — the handler body doesn't otherwise change:

```rust
#[debug_handler]
pub async fn search(
    QueryValidateWithMessage(params): QueryValidateWithMessage<CreateNote>,
) -> Result<Response> {
    format::json(params)
}
```

`QueryValidate`/`QueryValidateWithMessage` read the URL query string (e.g. `?title=abc&email=a@b.com`) regardless of the request's `Content-Type`.

## 4. What the client sees on failure

`JsonValidate`/`FormValidate`/`QueryValidate` (no `WithMessage`) return a bare `400`:

```json
{ "error": "Bad Request" }
```

The `*WithMessage` variants return the field-by-field detail, under an `errors` key, with **no** `error`/`description` set:

```json
{
  "errors": {
    "title": [
      { "code": "length", "message": "title must be at least 3 characters", "params": { "min": 3, "value": "ab" } }
    ],
    "email": [
      { "code": "email", "message": null, "params": { "value": "not-an-email" } }
    ]
  }
}
```

This is the same `Validation` branch of the [error → HTTP status map](/docs/reference/errors) (always `400`) — the `WithMessage` extractors populate `errors`, the plain ones map validation failures to a message-less `Error::BadRequest`.

Malformed input the extractor itself can't even deserialize (bad JSON, an unparsable query string) also returns `400`, before validation runs at all.

## 5. Validate without the `validator` crate

Implement `ValidatorTrait` directly for full control — useful if a rule doesn't fit `validator`'s derive macros:

```rust
use loco_rs::prelude::*;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, serde::Deserialize)]
pub struct CustomParams {
    pub name: String,
}

impl ValidatorTrait for CustomParams {
    fn validate(&self) -> Result<(), ModelValidationErrors> {
        if self.name.len() < 5 {
            let mut errors: BTreeMap<String, Vec<ValidationError>> = BTreeMap::new();
            let mut params: HashMap<String, serde_json::Value> = HashMap::new();
            params.insert("min".to_string(), serde_json::json!(5));
            errors.insert(
                "name".to_string(),
                vec![ValidationError { code: "length".to_string(), message: None, params }],
            );
            return Err(ModelValidationErrors { errors });
        }
        Ok(())
    }
}
```

`ValidationError` has three fields: `code: String`, `message: Option<String>`, `params: HashMap<String, serde_json::Value>`. Empty `params` are omitted from the JSON automatically. Any type implementing `ValidatorTrait` works with all six extractors above — the extractor doesn't care whether validation came from the `validator` crate or your own impl.

## Verify

```sh
curl -s -X POST localhost:5150/api/notes -H 'content-type: application/json' -d '{"title":"ab","email":"bad"}'
# {"errors":{"title":[...],"email":[...]}}   (JsonValidateWithMessage)
# {"error":"Bad Request"}                    (JsonValidate)
```

## Next

- [Handle errors](/docs/how-to/handle-errors) — the full error → status map and `CustomError`
- [Respond with different formats](/docs/how-to/respond-formats)
