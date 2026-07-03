# Spike S4 — H7 (`num-format`) & H8 (`axum-valid`)

Protocol: `docs/superpowers/audits/2026-07-03-code-quality/SPIKE-PROTOCOL.md`.
Spikes built at `scratchpad/spikes/h7-numformat/` and `scratchpad/spikes/h8-axumvalid/`
(throwaway cargo crates, not touching the loco workspace).

---

## H7 — `num-format` replacing the hand-rolled thousands-separator filter

**Incumbent:** `src/controller/views/tera_builtins/filters/number.rs:9-39`
(`separate_with_commas` / `separate_integer_part`), exercised by
`test_number_with_delimiter` (`number.rs:128-157`, 25 cases, 18 of which are
`Value::Number` — the only variant the delimiter logic actually touches;
`Value::String` cases fall through untouched at `number.rs:61`).

The incumbent operates purely on `value.to_string()` (the JSON number's own
textual form), splits on `.`, comma-groups only the **integer** part via a
manual byte-walk, and re-attaches the fractional part completely untouched —
it never parses the number into any numeric type, so it has no width limit.

### Library verified
`num-format = "0.4.4"` (confirmed via `cargo add`, crates.io latest as of
spike). Docs confirm: `ToFormattedString` (the `.to_formatted_string(&Locale)`
method) and the `Buffer`/`WriteFormatted` API **are only implemented for the
standard library's fixed-width integer types** (`i8`..`i128`, `u8`..`u128`).
There is no float/decimal formatting API and no arbitrary-precision integer
support in num-format.

### Spike design
Mirrored the incumbent's own architecture (split on `.`, format only the
integer-part substring, reattach the fractional part verbatim) but delegated
integer-part grouping to `num_format::ToFormattedString` instead of the
hand-rolled loop — i.e. gave num-format its best possible shot, not a naive
"parse the whole number as f64" approach.

```rust
fn format_integer_part(integer_part: &str) -> Result<String, String> {
    match integer_part.parse::<i128>() {
        Ok(n) => Ok(n.to_formatted_string(&Locale::en)),
        Err(e) => Err(format!("cannot parse integer part {integer_part:?} into i128: {e}")),
    }
}
```

### Compile + run
```
$ cargo run   # in scratchpad/spikes/h7-numformat
   Compiling num-format v0.4.4
   Compiling h7 v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.90s
     Running `target/debug/h7`
PASS  100 -> 100
PASS  100.2 -> 100.2
PASS  1000 -> 1,000
PASS  10000 -> 10,000
PASS  10000.1234 -> 10,000.1234
PASS  -100 -> -100
PASS  -100.2 -> -100.2
PASS  -1000 -> -1,000
PASS  -10000 -> -10,000
PASS  -10000.12345 -> -10,000.12345
PASS  0 -> 0
PASS  0.123 -> 0.123
PASS  1000000 -> 1,000,000
PASS  1000000000 -> 1,000,000,000
PASS  1234567890.123456 -> 1,234,567,890.123456
PASS  0.000123 -> 0.000123
FAIL  -0.123 -> got "0.123", expected "-0.123"
PASS  -1234567.89 -> -1,234,567.89

17 passed, 1 failed/errored out of 18

--- adversarial: integer part exceeding i128::MAX ---
num-format CANNOT handle it: cannot parse integer part "170141183460469231731687303715884105728123" into i128 for num-format: number too large to fit in target type
incumbent (string-only) handles it trivially: 170,141,183,460,469,231,731,687,303,715,884,105,728,123.456
```

### Findings

1. **The reviewer's stated risk (f64 routing) does not materialize** —
   num-format never needs the fractional part, and by splitting the string
   the same way the incumbent already does, no digit of the tested precision
   passes through `f64` at all. The `1_234_567_890.123456` case round-trips
   exactly because both implementations rely on the same `serde_json`
   `Number::to_string()`, not on num-format.

2. **A different, real regression was found instead:** `-0.123` is one of
   the incumbent's own tested cases (`number.rs:148`). Its integer part is
   the string `"-0"`. Parsed as `i128`, `"-0" == 0`, and `0i128` formats as
   `"0"` — **the sign is silently dropped**, producing `"0.123"` instead of
   `"-0.123"`. This is a genuine byte-for-byte failure on an existing,
   currently-passing incumbent test case: num-format's integer-typed API
   cannot distinguish "negative zero" (a valid textual number) from `0`,
   whereas the incumbent's string-only sign check (`num_str.starts_with('-')`)
   handles it trivially because it never converts to a number at all.

3. **Secondary, real limitation:** num-format's `ToFormattedString` requires
   parsing into a fixed-width type (max `i128`/`u128`). Any integer part
   wider than ~39 digits (a value the incumbent handles with zero extra code,
   since it's pure string grouping) simply cannot be formatted by num-format.
   Not covered by the current test suite, but it is the same class of
   "arbitrary decimal-string precision" the reviewer flagged as being at risk
   — it's just a fixed-width-integer ceiling rather than an f64 ceiling.

4. **Net LOC/dep is not a win even ignoring the bug.** Adopting num-format
   would still require: the same split-on-`.` logic the incumbent has, a new
   `i128`-parse-and-error-handle step (num-format has no infallible "just
   format this string" API), and a workaround for the `-0` sign-loss bug
   (re-detecting and re-prepending the sign from the string, since the parsed
   integer discards it) — i.e. *more* code than the 31-line incumbent, plus a
   new dependency, to reproduce strictly worse behavior.

### Verdict
**DOESN'T-FIT** — `num-format@0.4.4` — breaks an existing incumbent test
case (`-0.123` → `"0.123"`, sign dropped because num-format's integer-typed
API cannot represent negative zero) and additionally caps integer-part width
at i128 vs the incumbent's unbounded string grouping; requires *more* code
than the incumbent to even attempt the swap. Incumbent
(`src/controller/views/tera_builtins/filters/number.rs:9-39`) stays. Net LOC:
+dependency, code roughly flat-to-worse (≈0, not a reduction), for a
regression. This validates the incumbent's KPI7 score.

---

## H8 — `axum-valid` replacing the 6 `FromRequest` validate extractors

**Incumbent:** `src/controller/extractor/validate.rs` — 6 near-identical
`FromRequest` impls: `{Json,Form,Query} × {WithMessage (detailed), plain (simple)}`.

Confirmed the two response contracts by reading the incumbent end-to-end
(`validate.rs` + `src/errors.rs:146-147` + `src/validation.rs:66-78` +
`src/controller/mod.rs:133-157,228-248`):

- **Detailed tier** (`JsonValidateWithMessage`/`FormValidateWithMessage`/
  `QueryValidateWithMessage`, e.g. `validate.rs:50-54`): `Error::Validation`
  → HTTP 400, body:
  ```json
  {"errors": {"username": [{"code":"length","message":"...","params":{"min":3,"value":"ab"}}], "email": [...]}}
  ```
  (top-level `"errors"` wrapper key confirmed by `ErrorDetail` at
  `controller/mod.rs:237-244`, serializing `ModelValidationErrors.errors`.)

- **Simple tier** (`JsonValidate`/`FormValidate`/`QueryValidate`, e.g.
  `validate.rs:141-149`): validation errors are **deliberately discarded**
  (only logged via `tracing::debug!`) and `Error::BadRequest(String::new())`
  is returned → HTTP 400, body:
  ```json
  {"error": "Bad Request"}
  ```
  with no field names, codes, or messages leaked to the client at all
  (`ErrorDetail::new` omits the empty description entirely, confirmed by the
  incumbent's own test `test_json_validate_invalid`, `validate.rs:487-508`).

### Library verified
`axum-valid = "0.25.0"` (crates.io latest via `cargo add`), with `validator`
(the same backend loco uses), `json`, and `into_json` features enabled — the
`into_json` feature is required for axum-valid to emit a JSON body at all;
without it, the crate emits the rejection's `Display` as **plain text**, not
JSON, which would already fail both of Loco's JSON-shaped tiers.
Confirmed via source (`ValidationRejection::into_response`, gengteng/axum-valid
`main` branch): with `into_json`, the body is `axum::Json(v)` where `v` is
the raw `validator::ValidationErrors` — i.e. axum-valid does not wrap it in
any envelope; the shape is whatever `ValidationErrors`'s own `Serialize`
impl produces.

### Spike design
Built a real Axum router using `Valid<Json<TestUser>>` with the **exact same
struct and validation rules** as loco's own test fixture
(`validate.rs:311-317`, `TestUser { username: length(min=3), email: email }`),
drove it through `tower::ServiceExt::oneshot` with the same invalid payload
loco's own test uses (`validate.rs:391`), and inspected the real HTTP
response.

### Compile + run
```
$ cargo run   # in scratchpad/spikes/h8-axumvalid
   Compiling axum-valid v0.25.0
   Compiling h8 v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.35s
     Running `target/debug/h8`
=== axum-valid Valid<Json<TestUser>> on invalid payload ===
status: 400 Bad Request
body:   {"username":[{"code":"length","message":"username must be at least 3 characters","params":{"value":"ab","min":3}}],"email":[{"code":"email","message":"email must be valid","params":{"value":"invalid-email"}}]}

=== Verdict checks ===
axum-valid body has top-level "errors" wrapper key? false
axum-valid body matches Loco's SIMPLE tier ({"error":"Bad Request"} only)? false
```

### Findings

1. **Status code matches** (400, same as both Loco tiers) — but only because
   the `422` feature was left off; it exists and flips the default, so this
   is a config choice, not evidence of compatibility.

2. **Detailed tier: shape mismatch.** axum-valid's JSON body is the bare
   `validator::ValidationErrors` map (`{"username": [...], "email": [...]}`)
   with **no top-level `"errors"` wrapper key**. Loco's detailed tier wraps
   the same per-field data inside `{"errors": {...}}`
   (`controller/mod.rs:237-244`). Any client depending on the wrapper key
   (which is Loco's actual, tested, documented contract) would break. This
   is fixable only by writing a custom wrapping layer on top of axum-valid's
   `Valid` — i.e. more code than just using `Valid<Json<T>>` directly.

3. **Simple tier: not reproducible at all.** axum-valid's `ValidationRejection`
   has exactly one behavior on validation failure: serialize (or Display) the
   full `ValidationErrors`. There is no feature flag or extractor variant
   that discards validation detail and returns a generic message. Loco's
   simple tier exists specifically to avoid leaking field names/messages to
   the client while still logging them server-side
   (`tracing::debug!(err = ?err, ...)` then `Error::BadRequest(String::new())`,
   `validate.rs:143-146`). Reproducing that tier on top of axum-valid would
   require intercepting/mapping the rejection type before it reaches
   `IntoResponse` — i.e. re-implementing essentially the same
   `.validate().map_err(...)` logic the incumbent already has, on top of a
   new dependency, defeating the point of the swap for exactly the 3 impls
   (`JsonValidate`, `FormValidate`, `QueryValidate`) that most need
   simplifying.

4. **Net effect:** of the 6 incumbent impls, axum-valid's `Valid<Json/Form/Query<T>>`
   could directly replace at most the 3 "detailed" ones, and even then only
   after adding a wrapper to inject the missing `"errors"` key to match
   Loco's existing contract. The 3 "simple" ones cannot be expressed with
   axum-valid's extractor at all without writing custom rejection-handling
   code equivalent to what's already in `validate.rs` — so no net
   extractor-impl count is actually removed once you restore both tiers'
   exact contracts, while a new dependency is added.

### Verdict
**DOESN'T-FIT** — `axum-valid@0.25.0` — reproduces at most Loco's detailed
tier, and only with a body-shape gap (missing top-level `"errors"` wrapper
that `Error::Validation` / `controller/mod.rs:237-244` requires); the simple
tier (`{"error":"Bad Request"}` with validation detail deliberately
discarded, `validate.rs:141-149`) has no equivalent in axum-valid at all —
its rejection type always carries the full `ValidationErrors`. Reproducing
both tiers faithfully requires writing custom wrapping/discarding code on
top of axum-valid, which is not less code than the 6 impls in
`src/controller/extractor/validate.rs` (296 lines incl. tests, ~200 excl.),
plus a new dependency. Incumbent stays; validates its KPI7 score.
