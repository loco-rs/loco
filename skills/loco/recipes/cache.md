# Caching

`ctx.cache`. Never a `HashMap` in a `OnceLock`, never `moka` or `lru` directly —
Loco already wraps them.

---

## The trap: the default cache is a black hole

`CacheConfig` defaults to **`Null`**. A `Null` cache accepts every write, stores
nothing, and returns `None` from every read. Your code compiles, your tests
pass, and nothing is ever cached.

**`loco new` does not write a `cache:` block**, so a fresh app has a `Null`
cache until you add one — to **every** environment you care about, including
`config/test.yaml`:

```yaml
cache:
  kind: InMem
  max_capacity: 33554432      # bytes; optional, defaults to 32 MiB
```

If a cache "isn't working," check that this block exists before debugging your
keys.

`cache_inmem` is already in loco-rs's **default features** — `InMem` needs no
change to `Cargo.toml`. Redis does:

```toml
loco-rs = { version = "1.1", features = ["cache_redis"] }
```

```yaml
cache:
  kind: Redis
  uri: "{{ get_env(name='REDIS_URL', default='redis://127.0.0.1') }}"
```

Use `InMem` in development and test. Use `Redis` in production as soon as there
is more than one process — an in-memory cache is per-process, so two web workers
have two different caches and neither sees the other's entries.

---

## The API

All of it is on `ctx.cache`, all `async`.

| Method | Use |
|---|---|
| `get::<T>(key)` | `Ok(None)` on a miss — not an error |
| `insert(key, &value)` | store, no expiry |
| `insert_with_expiry(key, &value, duration)` | store with a TTL |
| `get_or_insert(key, fut)` | read, or compute-and-store on a miss |
| `get_or_insert_with_expiry(key, duration, fut)` | the same, with a TTL |
| `contains_key(key)` | presence without deserializing |
| `remove(key)` | invalidate one entry |
| `clear()` | invalidate everything |
| `ping()` | driver health |

Values are serialized, so `T` needs `Serialize` + `DeserializeOwned`. An entity
`Model` satisfies both.

## The normal shape

Read-through on a hot lookup, with a TTL:

```rust
impl Model {
    /// # Errors
    /// Returns `ModelError::EntityNotFound` when no such slug exists.
    pub async fn find_by_slug(ctx: &AppContext, slug: &str) -> ModelResult<Self> {
        let key = format!("link:{slug}");
        let found = ctx
            .cache
            .get_or_insert_with_expiry(&key, Duration::from_secs(300), async {
                links::Entity::find()
                    .filter(links::Column::Slug.eq(slug))
                    .one(&ctx.db)
                    .await?
                    .ok_or(ModelError::EntityNotFound)
            })
            .await?;
        Ok(found)
    }
}
```

The closure passed to `get_or_insert*` is a **future**, and its `Output` must be
the same `Result` type the call returns. A mismatch here produces a long,
unhelpful trait-bound error — see `errors.md`.

## Rules

- **Cache in the model, not the handler.** The handler asks for a link; whether
  that came from cache is the model's business. A handler that builds cache keys
  is a fat controller.
- **Namespace your keys** — `link:{slug}`, not `{slug}`. One flat keyspace is
  shared by everything in the app.
- **Invalidate on write.** The state transition that changes a row removes the
  key. A read-through cache with no invalidation serves stale data for a TTL,
  which is a bug you will find in production and not in tests.
- **Never cache a whole entity that carries secrets.** `users::Model` holds the
  password hash and API key; cache the id or a view struct instead.
- **A cache is not a database.** Anything you cannot recompute from the database
  does not belong in it — every entry can vanish at any time.
