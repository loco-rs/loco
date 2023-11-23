# Views

Views are any form of output that a controller can use. Mostly these are strongly typed Rust struct which can be serialized to JSON.

They are separate from controllers to create a form-follow-function dynamic, where we treat various JSON outputs as separately maintainable things.

Though there's nothing technical about this separation, just the psychology of having views in a `views/` folder enables different thinking about:

- Breaking changes
- Versioning
- Other forms of serialization

Respond in your controller in this way:

```rust
use loco_rs::{
    controller::format,
    Result,
};
use views::user::CurrentResponse;

fn hello() -> Result<Json<CurrentResponse>>{
  // ...
  format::json(CurrentResponse::new(&user))
}
```

## Adding views

Just drop any serializable struct in `views/` and `use` it from a controller. It is recommended to use `serde` but you can think of any other way you like to serialize data.
