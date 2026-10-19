# Judging rubric

You are grading two solutions to the same Loco task. One is a curated reference
written by a Loco maintainer; the other is an agent's attempt. **You are not
told which is which, and their order changes between runs — do not try to
guess.** Grade what is in front of you.

The doctrine above this section is the standard. Score against it, not against
general Rust taste.

## Criteria

Score **each submission** 1–5 on each criterion. Use the whole range: 3 means
"a competent developer who does not know Loco well," not "I am unsure."

**C1 — Framework usage (doctrine P1).**
- 1: reimplements what Loco ships, or adds dependencies for shipped capabilities.
- 3: uses some built-ins, hand-rolls others.
- 5: uses the framework's seam every time; any deviation is justified.

**C2 — Domain modeling (P2).**
- 1: all logic in the handler; the model file is an empty passthrough.
- 3: some extraction, but queries or rules still leak into the controller.
- 5: finders and transitions on the model; the handler is a thin orchestrator.

**C3 — Validation and configuration discipline (P3, P4).**
- 1: inline `if` checks in handlers and ad-hoc `std::env::var`.
- 3: mixed — declares validation but duplicates it, or reads one setting wrongly.
- 5: `Validatable` / `ActiveModelBehavior`, everything configurable in `config/`.

**C4 — Correctness under framework contracts (P5).**
- 1: violates invariants, leaks internals, ignores error semantics.
- 3: broadly correct with a real gap — a missing transaction, a wrong status code.
- 5: transactional where an invariant spans statements, typed errors, DTO
  boundaries respected.

**C5 — Would an expert Loco developer merge this?**
- 1: needs a rewrite.
- 3: needs a round of review comments.
- 5: merge as-is.

## Weighting the evidence packet

The evidence above is machine-collected fact, not judgement. "Added `lettre`"
is a fact; whether it is a defect is your call. Note especially that **the most
serious defects often leave no trace in the evidence** — a hand-rolled retry
loop adds no dependency, and serialising an entity with its password hash
touches no unusual file. Read the code.

## The gates are settled. Do not re-litigate them.

A submission reached you **only because `cargo build` and `cargo clippy -D
warnings` already passed on it.** That is a machine result, not an opinion.

So: **never claim a submission does not compile, will not compile, or is a
"compile risk."** If a call looks wrong to you — a trait you think is not in
scope, an argument you think is the wrong type — the compiler has already
disagreed, and you are working from a stale memory of an API rather than from
the code in front of you. Say nothing about it.

Two real examples from a previous run, both wrong, both stated confidently:

- "`vars.cli_arg("hours").unwrap_or("24")` does not compile, `cli_arg` returns
  `Result<&String>`" — it returns `Result<&str>`.
- "`.like(..)` requires `ExprTrait` in scope, only `Func` is imported, so this
  is a compile risk" — the submission had already built and passed
  `clippy -D warnings`. Whatever the judge believed about what was in scope,
  the compiler had already settled it.

Judge what the code *does*, which the gates cannot see. Whether it compiles is
not your question.

## Evidence discipline

Every score must be justified by something you can quote from the submission.
**If you cannot cite it, drop the claim.** Do not credit or penalise behaviour
you infer but cannot point to.

## Output

Respond with **only** a JSON object and no prose:

```json
{
  "a": {"c1": 3, "c2": 2, "c3": 4, "c4": 3, "c5": 3},
  "b": {"c1": 5, "c2": 5, "c3": 5, "c4": 4, "c5": 5},
  "verdict": "B",
  "margin": "decisive",
  "defects": [
    {
      "submission": "A",
      "summary": "Serialises the users entity directly, leaking password hash and api_key",
      "evidence": "src/controllers/x.rs :: format::json(user)"
    }
  ]
}
```

`margin` is one of `decisive`, `moderate`, `close`. List defects worst-first,
for the weaker submission primarily, but include any serious defect in either.
