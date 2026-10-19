# Eval run log

Every run, in order, including the bad ones. A score series is only worth
anything if the failures are in it — otherwise "we improved it" means "we kept
the run we liked".

Score is `compile rate` / `mean idiomatic (of 5)`, `with-skill` vs `bare`.

| # | cases | with-skill | bare | Δcompile | Δidiom | what changed since the previous run |
|---|---|---|---|---|---|---|
| 1 | 2 | 50% / — | 50% / — | 0 | +0.40 | first run; agentic, per-case calls |
| 2 | 2 | 0% / 4.80 | 50% / 4.30 | −50 | +0.50 | `produce` excluded the model file, so the model layer was un-scoreable |
| 3 | 2 | 50% / 4.80 | 50% / 4.70 | 0 | +0.10 | per-arm run dirs; artifacts captured |
| 4 | 2 | 50% / 4.90 | 0% / 4.50 | +50 | +0.40 | gave the skill arm `api-index.md` for the first time |
| 5 | 7 | 57% / 4.77 | 71% / 4.89 | **−14** | **−0.11** | corpus 2 → 7 cases |
| 6 | 7 | 86% / 4.94 | 86% / 4.94 | 0 | 0 | the three run-5 defect fixes (auth recipe, private-module paths, where-clauses) |
| 7 | 7 | 86% / 5.00 | 86% / 4.89 | 0 | +0.11 | Sea-ORM distilled wholesale; trait methods rendered for both crates |
| 8 | 7 | **100%** / 4.74 | 86% / 4.77 | **+14** | −0.03 | the two run-7 methodology fixes; `loco_rs::prelude` gained the Sea-ORM query traits |
| 9 | 7 | **100%** / 4.89 | 71% / 4.83 | **+29** | +0.06 | rubric forbids compile claims; skill gained the mailer/DSL/validator material. **Bare arm input byte-identical to run 8 — see below.** |
| 10 | 7 | 100% / 4.79 | 71% / 4.81 | +29 | **−0.03** | **not a new run** — run 9's answers rejudged in both orders. Same code, opposite sign. `spread` 0.09 mean / 0.60 worst |

## Run 5 — the honest null result

With enough cases to see anything, the skill measured *worse* on both axes.
Diagnosis, from the artifacts rather than from guessing:

- **`api-key-endpoint`**: skill arm failed, control passed. The auth recipe
  listed `auth::ApiToken` in a table with no example, so the agent omitted
  `State<AppContext>` and Axum could not infer the router state. The material
  actively misled. Fixed.
- **`user-search`**: skill arm wrote `model::query::paginate::PaginationQuery`.
  That path is in `api-index.md` and does not compile — `paginate` is a private
  module. **The anti-hallucination artifact produced a hallucination.** Fixed by
  whitelisting real modules instead of publishing rustdoc's definition paths.
- **`cached-stats`**: both arms failed on `Cache::get_or_insert_with_expiry`.
  The index rendered `f: F` and dropped the where-clause, so nothing said `F`
  had to be a `Future` with a specific `Output`. Fixed by rendering bounds.
- **Ceiling**: five of seven control answers scored a flat 5.00/5. There is
  almost no headroom for the skill to show anything.
- **Leakage**: every prompt carried a "context you can rely on" block handing the
  control the model layout and entity fields — a large share of what the skill
  teaches, given away free.

## Runs 6 and 7 — the aggregate hides the only real signal

Run 6 fixed the three defects run 5 exposed and both arms jumped to 86% / 4.94.
The delta was exactly zero, and 12 of 14 judge cells were a flat 5.00. Run 7
added the Sea-ORM index and moved the aggregate by +0.11 idiomatic and nothing
else. On the headline number the skill is worth almost nothing.

The per-case artifacts say something the headline cannot. One case,
`user-search`, fails in both arms — but **not for the same reason**, and the
difference is the whole point:

- **bare** (run 7, unchanged from run 5): `E0599` — `Expr::…like(&pattern)` with
  `sea_orm::ExprTrait` not imported. Sea-ORM's query surface is traits, and a
  model that has never read Sea-ORM does not know a method it can see in the
  docs needs a trait in scope. Same failure, three runs running.
- **with-skill** (run 7): `E0117` — an orphan-rule violation, `impl
  From<PageResponse<users::Model>> for UserSearchResponse` where the target is
  an alias of the foreign `Pager`. The Sea-ORM call site is correct.

So the distillation did kill the class it was aimed at, in the arm that has it,
and the remaining failure is a Rust reasoning error no framework index can fix.
The compile rate cannot show this: one failure is one failure.

**Two things this makes measurable, and neither is a scoring tweak:**

1. **The judge is saturated.** A rubric where the control scores 4.89/5 has
   0.11 of headroom. The mean is no longer the instrument; per-case failure
   *class* is. Runs 6 and 7 are the evidence, not an excuse — they were run and
   logged before this was written down.
2. **The prompts leak.** Every case carries a "context you can rely on" block
   handing the control arm the model layout and entity fields — a large part of
   what the skill exists to supply. The control is not a control.

Both fixes change what is measured, so they cannot be made quietly between runs
and reported as an improvement. They are listed here, unfixed, on purpose.

## Run 8 — the win is a framework fix, not a skill win

Both defects run 7 listed as unfixed are fixed: the prompts no longer carry a
hand-written context block (both arms get the app's source verbatim), and the
harness now reports the rustc error class per failure.

The headline moved — 100% vs 86% compile — and **the honest reading is that the
skill did not earn it.** Three runs running, the control failed `user-search`
on an unimported `ExprTrait`. That was never a knowledge gap the skill could
close; it was `loco_rs::prelude` re-exporting `QueryFilter` but not
`QueryOrder`, so `.filter()` compiled and `.order_by_desc()` did not. Loco was
working around it in its own generated scaffold. The prelude fix removes the
class **for every Loco app, skill or no skill** — and the control got most of
that benefit too.

What the control failure turned into is the more useful result. It is no longer
E0599; it is E0277:

```
`?` couldn't convert the error to `ModelError`
   the trait `From<loco_rs::Error>` is not implemented for `ModelError`
```

The bare arm wrote a model method returning `ModelResult` and called
`query::paginate`, which returns `loco_rs::Result`. The conversion exists in one
direction only. That is a real asymmetry in Loco's own API, and it surfaced only
because the failure-class reporting from this run makes "failed" distinguishable
from "failed the same way." It is now `errors.md` §6.

**The idiomatic delta is −0.03 and should not be read as a delta at all.** Two
of the four defects the judge charged against the with-skill arm are
verifiably false, and both are the same error:

- "`vars.cli_arg("hours").unwrap_or("24")` … does not compile, `cli_arg` returns
  `Result<&String>`" — it returns `Result<&str>`. Checked in `src/task.rs`.
- "`.like(..)` requires `ExprTrait` in scope; only `Func` is imported, so this
  is a compile risk" — `ExprTrait` is in the prelude as of this run.

Both submissions had already passed `cargo build` and `clippy -D warnings`
before reaching the judge, and the rubric hands it those results. It overrode a
machine fact with a stale memory of the API — twice, in one arm, confidently.
Worse, on `user-search` it scored the arm that **did not compile** (4.60) above
the arm that did (4.40). The rubric now forbids any claim about compilation.
That fix lands after this run's scores, so run 9 is the first to measure it.

Two further caveats, stated rather than buried:

- This run died partway on a full disk and was resumed. The with-skill answers
  were generated before several skill additions landed (the `ModelError` status
  table, the mailer `deliveries()` seam, the column-DSL notes), so **run 8 does
  not measure them.**
- The judge is still saturated: 9 of 14 cells are a flat 5.00.

## Run 9 — the run that measured the instrument instead of the skill

Headline: with-skill 100% / 4.89, bare 71% / 4.83. On its face the best result
in the series, +29 compile points. It is not, and the reason is the most
useful thing this eval has produced.

### The bare arm was an accidental replicate

Between the commit run 8 was scored at and the commit run 9 ran at, the only
files that changed were `RESULTS.md` and material under `skills/loco/`. **The
bare arm is handed no skill files.** Its prompt, its app source, and its task
list were byte-identical across the two runs. So runs 8 and 9 are two draws
from the same configuration — the control experiment nobody scheduled:

| bare arm, identical input | compile | mean idiomatic |
|---|---|---|
| run 8 | 86% (6/7) | 4.77 |
| run 9 | 71% (5/7) | 4.83 |

Two numbers follow, and both are damaging to the series:

- **Generation noise is at least one case.** A 14-point swing on the compile
  rate needs no change to anything. Run 8's celebrated "+14" is exactly this
  size. It was not a result.
- **End-to-end run noise is at least 0.06 on the mean.** Run 9's arm delta on
  idiomatic is +0.06 — the *same* number the control moved on its own. Every
  idiomatic delta in this table, all nine of them, is smaller than or equal to
  the drift between two identical runs.

  Note what this figure is. The bare arm *regenerated* in run 9, so the 0.06
  bundles two sources — the model writing different code, and the judge
  scoring it differently. It bounds the pair, not either one. Splitting them
  is what the new `spread` column is for: it rejudges the *same* submissions
  and so isolates the judge's own contribution.

Because n=7 and the outcome is binary, the compile rate cannot resolve
anything finer than 14.3 points. Two of the eight deltas above are exactly one
case; four are zero.

### The failure *class* was stable even though the case was not

The same two identical bare runs, per case:

- run 8: `user-search` fails, `E0277` — a `ModelResult` method calling a
  framework helper that returns `Result`.
- run 9: `user-search` fails on `E0034` — a different error entirely, and one
  I had introduced myself, dissected below — while `cached-stats` fails with
  `E0277`, the same asymmetry, in a different case.

The knowledge gap held still while the case exhibiting it moved. That is the
argument for failure-class reporting that run 7 made on a hunch; here it is
with a replicate behind it. It also means a per-case pass/fail table is the
wrong thing to diff between runs — the class is the stable unit, not the row.

### The instrument had a defect of its own

`eval.rs` documented, since it was written, that position bias was controlled
by "running each comparison twice with the submissions swapped and reporting
whether the two runs agreed." `README.md` said the same. **Neither was true.**
There was one judge call per arm, fixed order, every run. Nine runs of scores
carry no error bar and none can be reconstructed for them.

That is now implemented: each arm is judged in both orders and the gap prints
as `spread`, with the report stating outright when the arm delta fails to
clear it. It is a lower bound — the judge is told which side is the reference,
so the swap perturbs order only.

### What the run found that was real

Zero false compile claims this time, against two of run 8's four charged
defects: the rubric fix worked. And every substantive defect the judge named
checked out against source, which turned three of them into fixes:

1. **`Pager`/`PagerMeta` were missing from the prelude.** `query::paginate` is
   in it and returns a `PageResponse` whose `meta` field *is* a `PagerMeta` —
   but the pair you render it with was not reachable, so the arm hand-rolled
   an envelope with the same four fields. The eval's own curated reference had
   papered over this with a hand-written deep import, which is the tell. Fixed
   in the prelude; the reference now uses the prelude, so it is the regression
   test.
2. **`Config::settings::<T>()` did not exist.** Every app hand-deserialized
   `ctx.config.settings`, and the obvious spelling — `from_value(..).ok()` —
   cannot distinguish an absent block from a malformed one and silently runs
   on defaults. Added, with the malformed case pinned by a test.
3. **`errors.md` §6 was over-applied.** It says a model method calling a
   framework helper should widen its return type; the agent generalized that
   into `&AppContext` and `loco_rs::Result` on a method that only calls
   Sea-ORM's `count`. The section now states the boundary.

### And it caught a regression I had introduced

The bare `user-search` failure was `E0034` on `params.page.max(1)`:

```
multiple applicable items in scope — multiple `max` found
candidate #1 is defined in an impl of the trait `sea_orm::ExprTrait` for the type `T`
```

Run 8's prelude fix — the one this log credited with the +14 — exported
`ExprTrait`, which is `impl<T> ExprTrait for T where T: Into<Expr>`. Primitives
are `Into<Expr>`, so it reached every value in every file that globs the
prelude. Re-adding it to prove the point produces two failures:

- `n.max(1)` and `n.min(1)` stop compiling.
- `n.eq(&3)` keeps compiling and is **wrong** — `ExprTrait::eq` takes `self`
  where `PartialEq::eq` takes `&self`, so the by-value candidate wins with no
  ambiguity and yields an `Expr` instead of a `bool`.

`ExprTrait` is out of the prelude and `src/prelude.rs` now has tests that fail
if it returns.

**And the gap it was added to close never existed.** The E0599 that the bare
arm hit in runs 5, 6 and 7 was `Expr::col(users::Column::Name).like(&pattern)`
— the sea-query expression layer. `ColumnTrait::like` has always been on the
column itself, and `ColumnTrait` has always been in the prelude:
`users::Column::Name.like(&pattern)` compiles, and `errors.md` §2 already told
agents so. The submission had reached one layer too low; the prelude was fine.
I read a submission's mistake as a framework defect and widened the prelude to
accommodate it.

Run 8's commit did two things and they deserve opposite verdicts. Adding
`QueryOrder` was right — `.order_by_desc()` genuinely needs it, and Loco's own
generated scaffold was importing it by hand, which is the evidence that
convicts a prelude. Adding `ExprTrait` was wrong, and nothing in the codebase
was working around its absence. The tell I should have looked for and didn't:
**does the framework's own generated code work around this?** For `QueryOrder`
it did. For `ExprTrait` it never had.

So run 8's headline was, in the end, half a real fix and half a regression that
traded one failure class for a worse one, inside a metric too coarse to see
either.

## Run 10 — the same submissions, and the delta changes sign

Not a new generation. Run 9's saved answers, rejudged with the two-pass judge,
for $2.46. Nothing about the code differs. Only the judging does.

| on identical submissions | with-skill | bare | Δidiom |
|---|---|---|---|
| run 9, one pass | 4.89 | 4.83 | **+0.06** |
| run 10, mean of two passes | 4.79 | 4.81 | **−0.03** |

**The arm delta reversed sign without a line of code changing.** The with-skill
mean alone moved 0.10 between judgings of the same seven files — larger than
any arm delta in ten runs. Measured judge spread is 0.09 mean, 0.60 worst, and
the report now says so without being asked:

```
idiomatic delta    : -0.03 / 5
judge spread       : 0.09 mean, 0.60 worst (same code, both orders)
  -> |delta| 0.03 does not clear the 0.09 spread: NOT a result.
```

That closes the question the log has circled since run 5. **The mean idiomatic
score has never measured a difference between the arms.** Not "the effect is
small" — the instrument's own repeatability is coarser than every delta it has
ever reported. Runs 1–9's idiomatic column should be read as decoration.

The worst case is the instructive one. `nightly-digest`, with-skill, spread
0.60: judged submission-first the judge named **no** defects; judged
reference-first it named three, including a real one — `output: silent` in the
scheduler config discards the digest's output, so the report goes nowhere. Same
file, same rubric, one difference in reading order. So the order effect is not
a scoring wobble around a settled opinion; **it changes what the judge notices
at all.** Any future use of this judge should treat single-pass defect lists as
a sample, not an inventory.

### The compile column moved for a reason this time

`bare/user-search` went `E0034` → `E0599`. Run 10 replays run 9's exact answer
against the fixed prelude, so that change is attributable to one edit: removing
`ExprTrait`. The underlying code was always this —

```
error[E0599]: no method named `like` found for enum `Expr`
    Expr::expr(Func::lower(Expr::col(users::Column::Name))).like(pattern.as_str())
```

— an agent reaching for the sea-query expression layer. Exporting `ExprTrait`
did not fix that; it converted a clear "you need this trait" error into an
ambiguity error somewhere else in the file, and broke `.max()` for every Loco
app as the price. Removing it restores the honest diagnostic.

And that failure is the one place the skill demonstrably earns its keep. The
bare arm has reached for `Expr::` on this case in five of the last six runs.
`errors.md` §2 — *"reaching for `Expr::` usually means you have gone one layer
too low"* — is exactly that lesson, the with-skill arm compiles, and no
framework change was ever the right answer.

**So the honest scorecard after ten runs is:** the compile rate resolves one
case and shows a real, repeated gap on failure *class*; the idiomatic mean
resolves nothing; and the eval's most valuable output has not been either
number — it is the four framework defects it surfaced (`QueryOrder`, the
`Pager` gap, `Config::settings`, and the `ExprTrait` regression it caught one
run after I introduced it).

### The bill for the same change: a second self-inflicted regression

The `Pager` export written for this run was not gated, and
`controller::views::pagination` is `#[cfg(feature = "with-db")]`. Every DB-less
app therefore failed to compile `loco-rs` itself:

```
error[E0432]: unresolved import `crate::controller::views::pagination`
  --> src/prelude.rs:74:35
note: found an item that was configured out — gated behind the `with-db` feature
```

What matters is *which* gate saw it. `cargo test --all-features`, `loco-gen
--all-features`, workspace clippy `--all-features`, fmt, `agent-skill --check`
and `docs-syntax` were all green — and structurally could not have been
otherwise, because `with-db` is on in every one of them. **A `--all-features`
run is not a feature check.** `cargo hack check --each-feature` (17
configurations) catches it and is already in CI; the `loco-new` wizard matrix
is what actually caught it here, on the third starter combination.

That is now two regressions this workstream has introduced into the prelude in
three runs — `ExprTrait` and this one. The prelude is a glob import in every
file of every Loco app, and edits to it have a blast radius nothing else in the
crate has. The lesson is not "stop fixing the prelude"; the eval found four real
gaps there. It is that a prelude edit is not verified until the feature matrix
and the wizard matrix have both run on it.

## Discipline

Changes between runs are either **defect fixes the eval identified** (the three
above) or **stated methodology fixes**, made once and kept. Tuning cases until
the delta turns positive would fit the eval to the answer and make the number
worthless.
