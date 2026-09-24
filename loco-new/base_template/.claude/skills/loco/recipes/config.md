# Config and settings

`config/<env>.yaml` → `ctx.config`. **No `std::env::var` in application code**,
ever. This is stricter than Rails on purpose: Loco config is typed and
declarative, so an ad-hoc env read bypasses the one place a setting is supposed
to be discoverable.

---

## The environments

```
config/
  development.yaml     the default for `cargo loco start`
  test.yaml            used by the test harness
  production.yaml
```

Selected by `LOCO_ENV` (`development` when unset). A `<env>.local.yaml` beside
any of them is merged over it and is git-ignored — that is where a developer's
personal overrides go, not in the committed file.

## Your own settings

Anything the framework does not define goes under the free-form `settings:` key,
which reaches you as `ctx.config.settings` (an `Option<serde_json::Value>`).

`config/development.yaml`:

```yaml
settings:
  base_url: http://localhost:5150
  link_ttl_days: 30
```

`config/production.yaml`:

```yaml
settings:
  base_url: https://sho.rt
  link_ttl_days: 365
```

Do not read that `Value` field-by-field at each use site. Declare a typed struct
once and deserialize:

```rust
// src/data/settings.rs  (or src/models/settings.rs — anywhere in the app crate)
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub base_url: String,
    #[serde(default = "default_ttl_days")]
    pub link_ttl_days: u32,
}

const fn default_ttl_days() -> u32 { 30 }

impl Settings {
    /// # Errors
    /// When `settings:` is absent or does not match the schema.
    pub fn from_context(ctx: &AppContext) -> Result<Self> {
        let raw = ctx
            .config
            .settings
            .clone()
            .ok_or_else(|| Error::string("missing `settings:` in config"))?;
        Ok(serde_json::from_value(raw)?)
    }
}
```

Now a missing or misspelled key is one clear error at the point of use, rather
than a `None` that silently degrades three layers down.

## Secrets

Secrets belong in config too — via interpolation, not via a direct env read in
your code:

```yaml
database:
  uri: "{{ get_env(name='DATABASE_URL', default='sqlite://dev.sqlite?mode=rwc') }}"

settings:
  stripe_key: "{{ get_env(name='STRIPE_KEY') }}"
```

The template function is evaluated when config loads. Your code still just reads
`ctx.config`, so there is exactly one place that knows a value came from the
environment. `get_env` without a `default` fails loudly at boot if the variable
is unset — which is what you want for a required secret.

**Never commit a real secret to `config/production.yaml`.** Interpolate it.

## Reaching config from each place

| Where | How |
|---|---|
| handler | `State(ctx): State<AppContext>` → `ctx.config` |
| model method | take `&AppContext` rather than `&DatabaseConnection` when you need config |
| worker | `self.ctx.config` |
| task | the `AppContext` argument to `run` |
| mailer | the `&ctx` it is called with |

If a model method needs one setting, prefer passing the value in as an argument
over threading `AppContext` everywhere — the model stays testable and the
handler keeps the knowledge of where settings come from.

## A missing block is not an error

Config sections you never write do not fail — they fall back to an inert
default. `cache:` is the one that bites most often: `loco new` writes no
`cache:` block, so `ctx.cache` is a `Null` cache that accepts every write and
returns nothing, silently. See `recipes/cache.md`.

The general rule: if a subsystem "isn't working" and nothing is logged, check
that its block exists in the `config/<env>.yaml` for the environment you are
actually running — including `config/test.yaml`.

## Rules

- No `std::env::var` in application code. If you are about to write it, the
  value belongs in `settings:` with `get_env` interpolation.
- Every environment that needs a setting must define it. A key present only in
  `development.yaml` is a production boot failure or a silent default.
- Config is typed at the edge — deserialize `settings` into a struct once.
- `test.yaml` matters. A setting your tests need must be there too, or the
  request tests fail in a way that looks like a code bug.
