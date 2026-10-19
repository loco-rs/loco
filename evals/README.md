# Loco agent evals

Measures whether a coding agent writes *idiomatic* Loco — not just Loco that
compiles.

```sh
cargo xtask eval --list                        # the task corpus
cargo xtask eval --check-references             # gate every reference solution
cargo xtask eval --task nightly-digest          # run one task, both arms
cargo xtask eval                                # the whole suite
cargo xtask eval --no-judge                     # gates and cost only, no model calls
cargo xtask eval --resume                       # finish a run that died; do not re-bill generation
```

## Two harnesses, and which one to pay for

**The atomic corpus (this directory, `cargo xtask eval`) is the iteration
loop.** Seven scoped tasks against one pre-built app, answered in a single
tool-free call per arm. Cost is a few dollars and every task is an independent
data point.

**A greenfield build (`grade.py`) is the occasional integration check.** An
agent is turned loose to build a whole app from a spec, and `grade.py` scores
the result against four bars. It catches what the corpus structurally cannot —
whether the agent reaches for the *generators*, whether it wires things up,
whether the app boots — because the corpus hands it a wired app and asks for one
file.

The cost difference is not small, and it is why the corpus is the default:

| | atomic corpus | greenfield run |
|---|---|---|
| billed calls | 2 generation + 2 judge | one agent session, ~200 turns |
| measured cost | a few dollars | **$10–17 per arm** |
| data points | 7 per arm, independent | 1 |

Greenfield cost is dominated by context re-reads, not output: a measured run
billed 25.4M read tokens against 65k of output, because every turn re-sends the
whole conversation plus a ~34k system-prompt floor. Cost is therefore
superlinear in turn count, and turn count is what a greenfield build spends.
Stripping tool schemas was tested as a lever and is not one — it moved first-turn
context by 4%.

So: iterate on the corpus, and spend on a greenfield run only to check something
the corpus cannot see.

```sh
evals/grade.py <run-dir>    # scores a stored run; calls no model, so re-grading is free
```

`grade.py` is deliberately model-free — every check is a grep over the diff, the
command log, or a gate's exit code. That makes a stored run re-gradeable after
the fact, which is what makes correcting the grader cheap: five grader defects
were found and fixed by re-grading two stored runs, at no cost.

`evals/hidden/` holds behavioural tests copied into the app **after** the agent
stops. Nothing in there can be read or targeted while the app is being written,
so it measures behaviour no grep over the diff can see — that a click is
actually counted, that a listing is scoped to its owner.

## Why it is in this repo

An eval that lives somewhere else drifts from the thing it measures. The tasks
score against `skills/loco/doctrine.md` and the references compile against this
workspace's `loco-rs`; splitting them across repositories would reintroduce
exactly the divergence `cargo xtask agent-skill` exists to prevent.

Scratch apps and the shared build directory go to `target/eval/`, which is
already ignored.

## The two numbers

| KPI | What it measures | How |
|---|---|---|
| **Fluency** | is the code idiomatic | model judge, pairwise against the reference, criteria C1–C5 |
| **Tokens-to-green** | what rediscovery cost | output tokens and turns until the gates pass |

Tokens-to-green needs no judge and cannot be gamed. It is the direct measure of
an agent burning turns rediscovering the framework — which is the problem the
skill exists to remove — so it is reported even under `--no-judge`.

### What this instrument can and cannot resolve

Read a delta against these before calling it a result. Runs 8 and 9 happen to
be two draws of an identical bare configuration, so these are measured, not
assumed (`RESULTS.md`, run 9):

| | resolution |
|---|---|
| compile rate | **14.3 points** — one case out of seven. Nothing finer exists. |
| run-to-run noise, compile | **at least one case.** The identical control moved 86% → 71%. |
| run-to-run noise, idiomatic | **at least 0.06** on the mean. Bundles generation and judging — the control regenerated between those two runs. |
| judge-only noise | **0.09 mean, 0.60 worst**, measured on identical submissions (run 10). Printed every run as `spread`. |

Judging the *same* seven files a second time moved the arm delta from +0.06 to
−0.03. Not smaller — the other sign. Treat the idiomatic mean as decoration
until the corpus is large enough or hard enough to clear 0.09.

The order effect is not just noise around a settled opinion. On the worst case
the judge named zero defects reading one way and three reading the other, one
of them real. A single-pass defect list is a sample, not an inventory.

The consequence is worth stating plainly: **the mean idiomatic score has never
resolved a difference between the arms.** Every idiomatic delta in the log is
inside the drift between two identical runs. What the corpus has repeatedly
resolved is the *failure class* — which rustc error an arm dies on, and whether
the two arms die of the same thing. Weight that, not the means.

A corollary for anyone extending this: adding cases raises the resolution
(1/n), and the cheapest real improvement is cases the control actually fails.
Cases both arms pass at 5.00 cost money and measure nothing.

## How a run works

1. **Scaffold.** `loco new` with the task's flags. Apps are cached by flag
   signature and share one `CARGO_TARGET_DIR`; without that, every task pays a
   ~70s cold build and the loop is unusable.
2. **Two arms.** `with-skill` is the app exactly as `loco new` ships it.
   `bare` has `.claude/` and `AGENTS.md` removed — the control, and the state
   every pre-existing Loco app is in.
3. **Gates.** `cargo build`, `clippy -D warnings`, `cargo test`. These are
   admission control, not score: a submission that fails them never reaches the
   judge.
4. **Evidence.** Gate results, the dependency delta against a pristine app, and
   the file set written. Facts, not verdicts — handed to the judge as input.
5. **Judge.** Against the task's reference, **twice — once with the submission
   rendered above the reference, once below** — and the two averaged. The gap
   between them prints as `spread`, and it is the run's error bar: same code,
   same rubric, one difference in presentation. The report compares the arm
   delta against the mean spread and states outright when the delta does not
   clear it.

   Read runs 1-9 in `RESULTS.md` with that caveat: this file described the
   two-pass judge long before the harness did one, so those scores are single
   fixed-order draws and no error bar can be recovered for them.

## Why gates are not the score

`cargo clippy -D warnings` is an oracle most frameworks' evals do not get, and
it is worth using. But the defects that matter most leave no mechanical trace:

- a hand-rolled retry loop adds no dependency
- `format::json(user)` on an entity leaks the password hash and API key, and
  compiles, lints, and tests clean
- a check-then-insert without a transaction is a race, and passes every gate

All three are invisible to any rule you can write and obvious to a reader. So
mechanical checks decide *admission*; a model decides *quality*.

## Adding a task

```
evals/tasks/<id>/
  task.json      title, principles exercised, `loco new` flags
  PROMPT.md      exactly what the agent under test is asked to do
  reference/     curated solution, overlaid onto a fresh app
```

Two rules for a good task:

- **The reference must pass its own gates.** `--check-references` enforces it.
  Because every score is measured against the reference, a broken one corrupts
  the series silently rather than failing loudly.
- **Prefer a task with a trap the gates cannot see.** `user-search` exists
  because the `users` entity carries `password` and `api_key`, so the obvious
  wrong answer is fully green. That is the signal worth paying a judge for.

Write the reference by developing it against a real scaffolded app, not by hand.

## Determinism

The judge model is pinned in `xtask/src/eval.rs`. Changing it rebases every
score — a series spanning a model change is not a series. Change it only when
deliberately restarting the baseline.
