# 1.0.0 — reply drafts for the 4 newly-built issues

These four issues were re-scored (value + build-confidence) and **built for
1.0.0**. Post the reply and close when 1.0.0 ships. Commits are local on
`release/1.0.0`:

- `3f773108` feat(tls): Postgres + Redis TLS (#1191, #1341)
- `fb545ed9` feat(logger): init_layer/init_env_filter pub (#1753)
- `551e82fa` feat(db): typed db::dump + datetime dump fix (#1691, #1736)

---

## #1191 — Postgres TLS/SSL setup — CLOSE (fixed in 1.0.0)

> Thanks for the report — this is fixed in 1.0.0.
>
> Postgres TLS works straight from the connection URL, no code or feature flag
> needed: set `sslmode=require` (or `verify-ca` / `verify-full` with
> `sslrootcert=/path/ca.pem`) in `database.uri`. The rustls backend is already
> compiled in via Sea-ORM. There's a new how-to — "Connect to Postgres and
> Redis over TLS" — with copy-paste examples for RDS, Supabase and Neon.
>
> Note: the error you hit (`server does not support TLS`) is the server refusing
> the TLS negotiation, so double-check you're pointing at the provider's TLS
> endpoint. Closing as resolved in 1.0.0.

## #1341 — Redis over TLS — CLOSE (fixed in 1.0.0)

> Thanks — shipped in 1.0.0. Enable the new `redis_tls` Cargo feature and use a
> `rediss://` URL; it arms both the queue and cache Redis backends at once, uses
> webpki-bundled roots (works in distroless images), and stays on the pure-Rust
> `ring` provider so there's no C toolchain requirement. See the new "Connect to
> Postgres and Redis over TLS" how-to for ElastiCache/Upstash notes. Thanks
> @rustworthy for scoping it in the thread. Closing as resolved in 1.0.0.

## #1753 — make more internals public — CLOSE (partially addressed in 1.0.0)

> Thanks for the concrete example — `logger::init_layer` and
> `logger::init_env_filter` are now `pub` in 1.0.0, so you can reuse Loco's
> formatting and filter policy from a custom `Hooks::init_logger` and just add
> your `tracing_flame` (or OTLP, etc.) layer, instead of forking `init`.
>
> I deliberately kept this narrow rather than opening the whole surface at the
> 1.0 stability line — the banner decomposition you also mentioned is a real
> design task I'd rather do considered, in a later minor, than freeze now. If
> there are other specific internals you're blocked on, please open a focused
> issue naming them and the use case. Closing this one as addressed.

## #1691 — More reliable seed dumping — CLOSE (implemented in 1.0.0)

> Thanks for the detailed proposal and example — 1.0.0 ships essentially what you
> sketched. There's now a typed `db::dump::<A>()`, the counterpart to
> `db::seed::<A>()`: it streams rows and serializes each through its entity
> `Model` straight to a buffered writer, so memory is bounded to a single row
> and datetimes/UUIDs/JSON come back exactly as `seed` reads them. A new
> `Hooks::dump` backs `cargo loco db seed --dump` (default dumps every table via
> schema introspection; override it to call `db::dump` per entity for the
> streaming path). The SQLite boolean/datetime round-trip problems from #1627 /
> #1736 are fixed too. Closing as implemented in 1.0.0 — thanks for driving this.

## #1736 — Seeding from dumped fixture fails on SQLite — CLOSE (fixed in 1.0.0)

> Fixed in 1.0.0. Root cause: Loco's timestamptz columns default to
> `CURRENT_TIMESTAMP`, which SQLite stores as `"YYYY-MM-DD HH:MM:SS"` (space
> separator, no `T`/offset). The dump captured that verbatim, and re-seeding it
> into a typed `DateTimeWithTimeZone` model then failed chrono's RFC3339 parse —
> exactly the `Json("premature end of input")` you saw. Dumps now normalize
> those datetimes to RFC3339 (already-RFC3339 values are left untouched), so the
> reset → dump → seed flow round-trips. Thanks for the crisp repro. Closing as
> fixed in 1.0.0.
