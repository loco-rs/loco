# Spike S3 — "already a dependency or std-adjacent" drop-ins (H4, H5, H6)

Per `SPIKE-PROTOCOL.md`: real compiled throwaway crates, no changes to the loco
workspace. All three spikes live under
`/private/tmp/claude-501/-Users-jondot-projects-loco/cc99afe6-72c6-436d-babe-25c5f624e994/scratchpad/spikes/`:

- `h4-futures-fanout/`
- `h5-moka-future/`
- `h6-cookie-value/`

Loco version under test: workspace at `release/0.17.0`
(`Cargo.toml` — `futures-util = "0.3"` L126, `moka = "0.12.7"` (feature
`sync`) L148, `axum-extra = "0.10"` (feature `cookie`) L95).

---

## H4 — `futures_util::future::join_all` replacing serial fan-out loops in `mirror.rs` / `backup.rs`

### Incumbent

- `src/storage/strategies/backup.rs`: `upload` (L61-88), `delete` (L103-127),
  `rename` (L136-163), `copy` (L172-196), `upload_stream` (L217-258) — 5
  loops, each: serial `for secondary_store in secondaries { ... }`, collecting
  errors into `BTreeMap<String, String>` **after** the loop finishes, then
  `self.failure_mode.should_fail(&collect_errors)`.
- `src/storage/strategies/mirror.rs`: `upload` (L63-90), `delete` (L122-145)
  also check **after** the loop — but `rename` (L154-181), `copy`
  (L188-212), `upload_stream` (L250-291) check `should_fail` **inside** the
  loop, once per secondary, and `return Err(..)` immediately, before later
  secondaries are ever attempted.
- `FailureMode::should_fail` (backup.rs L273-283, mirror.rs L316-324) is a
  pure function of the resulting `BTreeMap<String,String>` — it doesn't care
  how the map was built.

That's 8 near-identical loops total (5 in backup.rs, 3 loop-shapes fully
duplicated + 2 shared in mirror.rs — 8 call sites across both files as named
in the brief).

### Spike

`h4-futures-fanout/src/main.rs`: models N secondary stores as async fns with
staggered artificial latency (10-50ms). Implements both the incumbent serial
loop and a `futures_util::future::join_all`-based concurrent fan-out, folds
concurrent results into the *same* `BTreeMap<String,String>` shape, and reuses
the *unmodified* `FailureMode::should_fail` against both maps.

Compiled with `cargo run --release` (moka/futures-util pulled from
crates.io, `futures-util = 0.3.32`, `tokio = 1.52.3`):

```
serial errors:     {"store_3": "store_3 failed", "store_5": "store_5 failed"} in 159.67375ms
concurrent errors: {"store_3": "store_3 failed", "store_5": "store_5 failed"} in 51.757125ms
BackupAll: serial.should_fail=true concurrent.should_fail=true
AllowBackupFailure: serial.should_fail=false concurrent.should_fail=false
AtLeastOneFailure: serial.should_fail=true concurrent.should_fail=true
CountFailure(2): serial.should_fail=true concurrent.should_fail=true
mirror.rs rename/copy in-loop-check incumbent short-circuits after processing index Some(3) of 5 secondaries -- concurrent fan-out would instead attempt ALL 5 secondaries before ever checking should_fail()
H4 spike OK: post-loop-check pattern (upload/delete/backup.rs) is concurrency-safe; in-loop-check pattern (mirror.rs rename/copy) is NOT semantically identical under concurrency.
```

### Findings

- For the **post-loop-check pattern** (all of backup.rs's 5 loops, plus
  mirror.rs's `upload`/`delete`) — concurrent fan-out via `join_all` produces
  a **byte-identical** `BTreeMap<String,String>` to the serial loop (`BTreeMap`
  equality doesn't care about insertion order, only content) and every
  `FailureMode` variant (`BackupAll`/`AllowBackupFailure`/`AtLeastOneFailure`/
  `CountFailure`) gives an identical verdict. Concurrency is a strict speed
  win here (~3x in the spike's staggered-latency scenario, and scales with
  the number of secondaries in real deployments) with **zero** semantic risk.
- The reviewer's RISK is real, but it doesn't apply uniformly: `mirror.rs`'s
  `rename`/`copy`/`upload_stream` check `should_fail` **inside** the loop and
  bail out immediately, meaning today (serially) later secondaries can be
  silently *skipped* once the failure mode already knows it will fail (e.g.
  `AtLeastOneFailure` short-circuits as soon as 2 failures are seen, leaving
  any secondaries after that point untouched — the spike shows this
  concretely: index 3 of 5, secondaries at index 4 never attempted). A naive
  `join_all` conversion of this specific pattern **always runs every
  secondary concurrently before checking**, which is a genuine behavior
  change (more work done, more side effects triggered, e.g. more writes to
  secondaries that would otherwise have been left untouched) for that one
  sub-pattern — not because of concurrency ordering, but because the
  incumbent's mid-loop early-return was already inconsistent with the rest
  of the file (mirror.rs's own `upload`/`delete` don't do this; backup.rs
  never does this). This is a pre-existing internal inconsistency in Loco,
  not something the swap invents.
- Net LOC: the 8 duplicated serial loops (`for secondary_store in
  secondaries { match storage.as_store_err(...) { ... } }`, ~12-18 lines
  each) collapse to one shared helper of ~15 lines
  (`build_futures_and_collect_errors` style) called from each strategy
  method — netting roughly **-40 to -60 LOC** across `mirror.rs` + `backup.rs`
  combined, plus removing the duplication smell (same match-arm shape
  repeated 8x). `futures-util` is already a direct dependency
  (`Cargo.toml:126`), so no new dependency is introduced.

### Verdict

`PARTIAL` — concurrent fan-out is a proven, safe drop-in for the majority
pattern (backup.rs's 5 loops + mirror.rs upload/delete: 6 of 8 sites), with
identical error-collection semantics and a real LOC/perf win. But for
mirror.rs's `rename`/`copy`/`upload_stream` (the in-loop early-return
pattern, 3 of 8 sites), a literal `join_all` swap changes behavior: it always
executes every secondary op before short-circuiting, instead of skipping
later secondaries once failure is already certain. Recommend the swap for
all 8 sites, but call out and consciously accept (or explicitly preserve via
a documented decision) the behavior change in the 3 in-loop-check sites —
this is arguably a *bug fix* (removing an inconsistency) rather than a
regression, but it must be a deliberate choice, not a silent side effect of
the refactor.

---

## H5 — `moka::future::Cache` replacing `moka::sync::Cache` in the in-memory cache driver

### Incumbent

`src/cache/drivers/inmem.rs` wraps `moka::sync::Cache<String, (Expiration,
String)>` (L10, L24-27) behind the `async_trait` `CacheDriver` (L49-137). All
six trait methods (`ping`, `contains_key`, `get`, `insert`,
`insert_with_expiry`, `remove`, `clear`) are declared `async fn` but their
bodies call the purely synchronous `moka::sync::Cache` API
(`.contains_key()`, `.get()`, `.insert()`, `.remove()`, `.invalidate_all()`)
— i.e. "async" only at the trait-signature level, never actually yielding.
Custom `Expiry<String, (Expiration, String)>` impl (`InMemExpiry`,
L155-166) drives per-entry TTL via `expire_after_create`.

### Spike

`h5-moka-future/src/main.rs`: rebuilds `Inmem` verbatim against
`moka::future::Cache` (moka `0.12.7` requested, resolved to `0.12.15`,
feature `future`), reusing the *identical* `Expiry` trait impl unchanged
(the `Expiry` trait is shared between `moka::sync` and `moka::future` — same
signature, no adaptation needed), and exercises every op `inmem.rs` uses,
plus `get_with` (mentioned in the brief, not currently used by `inmem.rs`
but relevant for future compute-on-miss use).

```
$ cargo run --release
   Compiling moka v0.12.15
   ...
H5 spike OK: moka::future::Cache (0.12.7, feature = "future") covers contains_key/get/insert/insert_with_expiry/remove/clear/Expiry, all natively async, plus get_with for future compute-on-miss use.
```

All assertions passed, including: TTL expiry via `insert_with_expiry` +
`run_pending_tasks().await` (future::Cache needs an explicit maintenance-task
poke or a subsequent op to force eviction visibility, same as sync::Cache's
`invalidate_all`/get-triggered cleanup — no new behavior here), clear-all via
`invalidate_all()` (present on `future::Cache` too, itself sync — no `.await`
needed since it just marks entries for background eviction), and `get_with`
for future use.

### Findings

- `moka::future::Cache` covers 100% of what `inmem.rs` uses today:
  `contains_key`, `get`, `insert`, `remove`, `invalidate_all`, plus
  `Cache::builder().max_capacity().expire_after(Expiry)`. The `Expiry` trait
  itself is generic over both cache variants — zero adaptation cost for the
  TTL logic.
- The change is a straight `Cache<K,V>` type swap
  (`moka::sync::Cache` → `moka::future::Cache`) plus adding `.await` after
  `get`/`insert`/`remove` calls (they become real async operations — `get`
  can synchronize with in-flight `get_with` computations, `insert`/`remove`
  enqueue through moka's internal command channel rather than blocking
  in-thread). This removes the "sync API behind an async trait" smell
  exactly as hypothesized: the driver's `async fn` signatures stop being
  cosmetic.
- Dependency-wise, `moka = { version = "0.12.7", features = ["sync"] }`
  (`Cargo.toml:148`) just needs `features = ["future"]` (or both, if
  something else in the crate still needs sync — nothing else in the loco
  codebase uses `moka` outside `inmem.rs`, per grep). No new external
  dependency — `moka` is already pinned.
- LOC impact is roughly a wash (~0 net LOC): trait method bodies gain
  `.await`, lose nothing. The win is architectural correctness, not line
  count.

### Verdict

`PROVEN-FIT` — `moka::future::Cache` (`moka@0.12.7`, feature `future`)
compiles as a drop-in replacement for every operation `inmem.rs` uses, with
the identical `Expiry`-based TTL mechanism carried over unchanged. It removes
the sync-behind-async smell (the async trait becomes genuinely async) with
~0 net LOC and no new dependency (moka is already pinned; only the enabled
feature set changes from `sync` to `future`, or `sync,future` if both are
needed elsewhere during migration). Recommend.

---

## H6 — `Cookie::value()` replacing hand-rolled cookie reparse in the JWT extractor

### Incumbent

`src/controller/extractor/auth.rs:224-234`, `extract_token_from_cookie`:

```rust
pub fn extract_token_from_cookie(name: &str, parts: &Parts) -> LocoResult<String> {
    let jar: cookie::CookieJar = cookie::CookieJar::from_headers(&parts.headers);
    Ok(jar
        .get(name)
        .ok_or(Error::Unauthorized("token is not found".to_string()))?
        .to_string()
        .strip_prefix(&format!("{name}="))
        .ok_or_else(|| Error::Unauthorized("error strip value".to_string()))?
        .to_string())
}
```

`jar.get(name)` already returns a typed `&axum_extra::extract::cookie::Cookie`
(re-exported `cookie::Cookie`) — but instead of calling its typed
`.value()` accessor, the incumbent calls `.to_string()` (which re-serializes
the whole cookie as `name=value[; attrs...]` via `Display`), then manually
strips the `"{name}="` prefix back off with `strip_prefix`, with a second
fallible `ok_or_else` for the strip. `axum_extra::extract::cookie` is
already imported at `auth.rs:29`.

### Spike

`h6-cookie-value/src/main.rs`: built against `axum = "0.8"` /
`axum-extra = { version = "0.10", features = ["cookie"] }` (resolved to
`axum 0.8.9` / `axum-extra 0.10.3` / `cookie 0.18.1` — matching Loco's pin).
Reimplements the incumbent verbatim alongside a candidate using
`.value().to_string()`, and runs both against: a plain token, a token
containing extra `=` characters (adversarial case for the strip-based
approach), a quoted RFC-6265 cookie value, and a missing-cookie case.

```
$ cargo run --release
   Compiling axum-extra v0.10.3
   ...
plain: incumbent="cookie_value_123" candidate="cookie_value_123"
multi-equals: incumbent="abc=def=ghi" candidate="abc=def=ghi"
quoted: incumbent=Ok("\"quoted-value\"") candidate=Ok("\"quoted-value\"")
H6 spike OK: Cookie::value() (axum-extra 0.10, cookie feature) matches the hand-rolled reparse for all normal token shapes.
```

### Findings

- `Cookie::value()` exists and is stable in `axum-extra 0.10.3` (re-exporting
  `cookie 0.18.1`'s `Cookie::value()`), the exact version Loco pins
  (`Cargo.toml:95`). API is real, not hallucinated.
- Every test case — including the adversarial multi-`=` value and a quoted
  value — returns byte-identical results between the incumbent's
  serialize-then-strip approach and the direct `.value()` accessor. This
  makes sense: `.value()` reads the parsed field directly; the incumbent's
  `.to_string()` re-serializes that same parsed state and then undoes its
  own serialization by stripping the prefix back off — it's strictly more
  work to reach the same data `.value()` already exposes.
- Replacing the 5-line stringify-strip-restring dance with
  `jar.get(name).ok_or(...)?.value().to_string()` removes one fallible step
  (`strip_prefix(...).ok_or_else(...)`) entirely — that failure branch is
  dead code in practice (given `Cookie`'s own `Display` impl always starts
  with `"{name}="`, the strip cannot fail for any cookie actually returned by
  `jar.get(name)` — it only existed to complete the round-trip the code
  invented for itself).
- Net LOC: incumbent is 11 lines (224-234); candidate is 6 lines. **-5 LOC**,
  one fewer fallible branch, zero new dependencies (already imported).

### Verdict

`PROVEN-FIT` — `axum_extra::extract::cookie::Cookie::value()` (axum-extra
`0.10.3` / cookie `0.18.1`, matching Loco's pinned `axum-extra = "0.10"`,
`Cargo.toml:95`) is a strictly simpler, behavior-identical replacement for
the hand-rolled `.to_string()` + `strip_prefix` reparse in
`extract_token_from_cookie` (`auth.rs:224-234`). Net **-5 LOC**, removes one
now-provably-dead fallible branch (the strip-prefix failure case), no new
dependency. Trivial, safe win — implement.

---

## Summary verdicts

- `PARTIAL — futures-util@0.3.32 (join_all) — safe & LOC-saving for 6 of 8 fan-out loops (post-loop-check pattern); the 3 in-loop-check loops in mirror.rs rename/copy/upload_stream lose a real early-exit skip-later-secondaries behavior that must be a deliberate call, not silent — incumbent @src/storage/strategies/{mirror,backup}.rs, net LOC ~-40 to -60`
- `PROVEN-FIT — moka@0.12.7 (future::Cache, feature "future") — drop-in for every inmem.rs op incl. Expiry-based TTL, removes sync-behind-async smell, no new dep — incumbent @src/cache/drivers/inmem.rs, net LOC ~0`
- `PROVEN-FIT — axum-extra@0.10.3 (Cookie::value()) — byte-identical output incl. adversarial multi-"=" and quoted-value cases, removes a dead fallible branch, already imported — incumbent @src/controller/extractor/auth.rs:224-234, net LOC -5`
