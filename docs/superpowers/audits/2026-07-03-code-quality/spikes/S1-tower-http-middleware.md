# Spike S1 — tower-http / axum-client-ip replacing hand-rolled request-id and remote-ip middleware (H1, H2)

Per `SPIKE-PROTOCOL.md`: real compiled throwaway crates, no changes to the loco
workspace. Both spikes live under
`/private/tmp/claude-501/-Users-jondot-projects-loco/cc99afe6-72c6-436d-babe-25c5f624e994/scratchpad/spikes/`:

- `h1-tower-http-request-id/`
- `h2-axum-client-ip/`

Loco version under test: workspace at `release/0.17.0`. Incumbents:
`src/controller/middleware/request_id.rs` (137 lines) and
`src/controller/middleware/remote_ip.rs` (349 lines). Loco's `Cargo.toml`
(L119, L186-194) already depends on `tower-http = "0.6.8"` with features
`["trace", "catch-panic", "timeout", "add-extension", "cors", "fs",
"set-header", "compression-full"]` — **not** `request-id`. `axum-client-ip` is
not a dependency at all today.

Versions verified live via docs.rs/crates.io during this spike (not assumed):
`tower-http` latest is **0.7.0** (crates.io API, published 2026-06-15; Loco is
on 0.6.8, one minor behind but `request-id`/`uuid` features exist on both
lines). `axum-client-ip` latest is **1.3.1** (crates.io API, published
2026-01-22), which internally delegates to the `client-ip` crate (0.2.1, seen
in the actual `cargo build` dependency graph below).

---

## H1 — `tower_http::request_id` (`SetRequestIdLayer` + `PropagateRequestIdLayer` + custom `MakeRequestId`) replacing `request_id.rs`

### Incumbent behavior (read in full, `src/controller/middleware/request_id.rs`)

`make_request_id` (L100-114): given an optional incoming `x-request-id`
header,
1. strip every character not matching `[\w\-@]` (regex `ID_CLEANUP`, L26),
2. truncate to 255 chars (`MAX_LEN`, L19),
3. if the result is **empty** (header absent, empty, or fully-invalid, e.g.
   `"=========="` per its own test at L129), **fall back to a fresh
   `Uuid::new_v4()`**,
4. otherwise use the sanitized value.

The result is stored in request extensions (`LocoRequestId`) and written back
onto the **response** `x-request-id` header (L80-97). One
`axum::middleware::from_fn` layer (L58) does all of it.

### Spike (`h1-tower-http-request-id/`)

`tower-http = { version = "0.7.0", features = ["request-id", "trace",
"util", "uuid"] }`, real API fetched from
`docs.rs/tower-http/0.7.0/src/tower_http/request_id.rs.html`:

```rust
pub trait MakeRequestId {
    fn make_request_id<B>(&mut self, request: &Request<B>) -> Option<RequestId>;
}
```

`SetRequestId::call` source (quoted directly from docs.rs, not assumed):

```rust
fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
    if let Some(request_id) = req.headers().get(&self.header_name) {
        if req.extensions().get::<RequestId>().is_none() {
            let request_id = request_id.clone();
            req.extensions_mut().insert(RequestId::new(request_id));
        }
    } else if let Some(request_id) = self.make_request_id.make_request_id(&req) {
        req.extensions_mut().insert(request_id.clone());
        req.headers_mut().insert(self.header_name.clone(), request_id.0);
    }
    self.inner.call(req)
}
```

**Finding #1 (semantic gap):** `make_request_id` is invoked **only when the
header is absent**. If a header is present — even empty, even 1000 chars,
even full of `=` characters — `SetRequestId` copies it through **verbatim,
unsanitized, untruncated**. A bare `MakeRequestId` impl cannot reproduce
Loco's sanitize-or-fallback rule; you need an extra pre-pass middleware that
sanitizes the incoming header (and removes it entirely if sanitization
yields empty, so the "absent" branch fires and a UUID gets generated). That
pre-pass ends up being *almost the same code Loco already has* — the same
regex, the same truncate, the same empty check.

**Finding #2 (real footgun, discovered by the spike literally failing
twice):** layering order is non-obvious and got it wrong twice before tests
passed. `PropagateRequestId::call` (also quoted from docs.rs) captures the
request's **current header value at its own `call()`-time**:

```rust
fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
    let request_id = req.headers().get(&self.header_name).cloned().map(RequestId::new);
    PropagateRequestIdResponseFuture { inner: self.inner.call(req), header_name: self.header_name.clone(), request_id }
}
```

`Router::layer` makes the *last* `.layer()` call the outermost layer (runs
first on the request path). The first spike attempt added
`SetRequestIdLayer` then `PropagateRequestIdLayer` (mirroring tower-http's own
generic doc example) — under `Router`, that makes `PropagateRequestIdLayer`
outermost, so it captures the request header **before** `SetRequestId` ever
runs. Result: for the no-incoming-header case (the single most common case in
production), the response *silently never got an `x-request-id` header at
all* — confirmed by a failing `cargo test` run:

```
test tests::no_header_generates_uuid ... FAILED
thread 'tests::no_header_generates_uuid' panicked: x-request-id must be set on response
```

Fixing it requires the *opposite* of the naive/documented order — `Propagate`
must be layered before (more inward than) `Set`, which must be layered before
the sanitize pre-pass. After correcting the order and adding the sanitize
pre-pass, all 8 tests pass, reproducing every one of Loco's own snapshot
cases (dirty header stripped, empty/all-separator header falls back to UUID,
overlong header truncated to 255, valid header passed through, no header
generates a UUID):

```
$ cargo test
running 8 tests
test tests::without_sanitize_layer_header_passes_through_verbatim ... ok
test tests::no_header_generates_uuid ... ok
test tests::bare_layers_no_header_at_all_debug ... ok
test tests::valid_header_is_propagated_unchanged ... ok
test tests::overlong_header_is_truncated_to_255 ... ok
test tests::empty_header_falls_back_to_uuid ... ok
test tests::dirty_header_is_sanitized ... ok
test tests::all_separators_header_falls_back_to_uuid ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### LOC comparison

Incumbent, non-test/non-blank/non-comment lines: **72** (struct + `impl
MiddlewareLayer` + `LocoRequestId` wrapper + `request_id_middleware` +
`make_request_id`). Spike, same measure (imports, sanitize pre-pass fn,
router wiring, excluding `main`/tests): **53**. A naive raw-LOC read says
"spike is smaller," but that number hides the real trade:

- The sanitize logic — the actual hard/valuable part of the incumbent — is
  **not eliminated**, just relocated into an equivalent custom
  `axum::middleware::from_fn` pre-pass. Net new logic added: ~0.
- What tower-http actually buys you is the response-propagation boilerplate
  (~10 LOC in Loco today) — a modest win.
- What it costs: two additional layers to reason about (`SetRequestIdLayer`,
  `PropagateRequestIdLayer`), a documented-but-easy-to-get-backwards ordering
  requirement that silently breaks the majority case (no header) when wrong,
  and enabling two new Cargo features (`request-id`, `uuid`) on top of the
  already-present `tower-http` dependency.
- No *new* crate dependency is required (tower-http is already a Loco
  dependency) — that part of the hypothesis is directionally true — but "no
  new dependency" isn't the same as "simpler."

### Verdict

`DOESN'T-FIT` — the swap doesn't remove Loco's hardest logic (the sanitize
rule must be reimplemented nearly verbatim regardless), and it introduces a
non-obvious layering-order requirement that, done the "obvious"/documented
way, silently drops the request ID on the most common request path (no
incoming header) — a regression, not an improvement. Net LOC is roughly flat
once the required sanitize pre-pass and correct 3-layer wiring are counted;
the complexity/fragility goes up, not down. The incumbent's single
`from_fn` middleware stays; this validates its current design.

---

## H2 — `axum-client-ip` replacing `remote_ip.rs`

### Incumbent behavior (read in full, `src/controller/middleware/remote_ip.rs`)

`maybe_get_forwarded` (L128-178), quoting the module's own doc comment
(L92-94, citing MDN's "Selecting an IP address" / "Trusted proxy list"
algorithm):

> The X-Forwarded-For IP list is searched from the rightmost, skipping all
> addresses that are on the trusted proxy list. The first non-matching
> address is the target address.

Concretely, from the incumbent's own `#[cfg(test)]` cases (L301-349):

- `xff("51.50.51.50,10.0.0.1,192.168.1.1")` → `Some(51.50.51.50)` — skips
  **two** trusted-proxy hops (`10.0.0.0/8` and `192.168.0.0/16`, both in the
  default trusted list, L39-51).
- `xff("19.84.19.84,192.168.0.1")` → `Some(19.84.19.84)` — skips **one**
  trusted hop.
- A custom `trusted_proxies` list (config `RemoteIpMiddleware.trusted_proxies`,
  `RemoteIP` struct) **replaces** (not appends to) the built-in list.

This is a genuine N-hop, proxy-whitelist-aware algorithm — the entire
point of the module, stated explicitly in its own module docs (L55-94) and
its security warning ("IN THE WRONG ARCHITECTURE IT CAN MAKE YOU VULNERABLE
TO IP SPOOFING", L63-64).

### Spike (`h2-axum-client-ip/`)

`axum-client-ip = "1.3"` (resolved to 1.3.1 in `Cargo.lock`, pulling in
`client-ip = "0.2.1"` per the real `cargo build` dependency graph). Real
source of the extractor it uses, `RightmostXForwardedFor`, fetched from
`docs.rs/client-ip/latest/src/client_ip/lib.rs.html`:

```rust
pub fn rightmost_x_forwarded_for(header_map: &HeaderMap) -> Result<IpAddr> {
    fn ip_from_header_value(header_value: &str) -> Result<IpAddr> {
        header_value.split(',').next_back()...trim().parse::<IpAddr>()...
    }
    ...
}
```

It **unconditionally takes the single last comma-separated entry.** There is
no trusted-proxy list, no skipping, no "how many hops do I trust" concept at
all. axum-client-ip's own changelog/docs (fetched via WebFetch) confirm this
is deliberate: version 1.0 "Removed `InsecureClientIp` and related 'leftmost'
IP logic. The library now focuses solely on secure extraction based on
trusted headers" — i.e. it assumes you configure it to read the *one* header
your *one* trusted edge proxy sets, not that it walks a chain skipping
several trusted hops.

Compiled spike, real test run against Loco's own two multi-hop test vectors:

```
$ cargo build
   Compiling client-ip v0.2.1
   Compiling axum-client-ip v1.3.1
   Compiling h2-axum-client-ip v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.19s

$ cargo test
running 3 tests
test tests::zero_hop_case_both_approaches_agree ... ok
test tests::one_trusted_hop_loco_finds_real_client_axum_client_ip_does_not ... ok
test tests::two_trusted_hops_loco_finds_real_client_axum_client_ip_does_not ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Concretely: for `xff("51.50.51.50,10.0.0.1,192.168.1.1")`, Loco returns
`51.50.51.50` (the real client); `axum-client-ip`'s `RightmostXForwardedFor`
returns `192.168.1.1` — literally a private/trusted-proxy address, misreported
as "the client IP." Same divergence, one hop, for
`xff("19.84.19.84,192.168.0.1")` → axum-client-ip returns `192.168.0.1`
instead of `19.84.19.84`. The two approaches **only** agree in the
degenerate zero-trusted-hop case (`xff("192.1.1.1")` → both return
`192.1.1.1`), which is the *one* case where "just take the rightmost value"
happens to be indistinguishable from "walk back skipping trusted proxies."

### Verdict

`DOESN'T-FIT` — this is not an edge-case nuance, it's the incumbent's core
purpose. `remote_ip.rs` exists specifically to handle deployments with more
than one trusted hop (CDN + load balancer + ingress, a completely normal
production topology) by walking the `X-Forwarded-For` chain from the right
and skipping every address that matches a configurable trusted-proxy
CIDR list. `axum-client-ip` 1.x deliberately dropped that model in favor of
"pick one header shape for your one trusted edge and trust it wholesale" —
its own docs confirm this was an intentional design choice, not an oversight.
Swapping it in for any Loco deployment with more than one trusted proxy hop
silently misattributes a proxy's own address as the client's, which is a
security-relevant regression, not a simplification. The incumbent's
349-line hand-rolled `tower::Service` — trusted-proxy-list, rightmost-skip,
configurable-replacement-list and all — stays; this strongly validates its
KPI7 score. (`trusted_proxies` config replacement, custom CIDR ranges, and
the `RemoteIP::{Forwarded,Socket,None}` distinction have no equivalent in
axum-client-ip at all, reinforcing the gap.)

---

## Summary

| H | Library@ver | Verdict | Reason |
|---|---|---|---|
| H1 | `tower-http@0.7.0` `request_id` module | `DOESN'T-FIT` | Sanitize logic must be reimplemented nearly verbatim (not eliminated); correct layering order is non-obvious and the documented/naive order silently drops the ID on the no-header (majority) case |
| H2 | `axum-client-ip@1.3.1` (+ `client-ip@0.2.1`) | `DOESN'T-FIT` | No trusted-proxy list / multi-hop skip semantics at all (removed by design in 1.0); diverges from Loco on both of Loco's own multi-hop test vectors, misattributing a trusted proxy's address as the client IP |
