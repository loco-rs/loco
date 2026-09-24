---
title: "Ten eval runs and an honest null"
description: "We built an eval to prove the Loco skill makes agents better. Ten runs later, the headline number never moved. What moved was which mistakes agents made. Part 4 of teaching agents Loco."
pubDate: 2026-10-08
authors:
  - team-loco
---

A skill you can't measure is a vibe. So we built `cargo xtask eval`: seven
tasks against a new Loco app from `loco new`, each with a reference solution we
wrote and tested. One agent gets the app as `loco new` ships
it, skill included. The other gets the same app with `.claude/` and `AGENTS.md`
deleted, which is the state every existing Loco app is in.

Rust gives us a grader for free. Every answer has to pass `cargo build`,
`clippy -D warnings` and `cargo test` before anything else looks at it. On top
of that, a model judge compared each answer to the reference for idiomatic
Loco.

We logged every run in
[`evals/RESULTS.md`](https://github.com/loco-rs/loco/blob/master/evals/RESULTS.md),
bad ones included. A score series is only worth something if the failures are
in it.

## What the numbers said

| run | with skill | without | what changed |
|---|---|---|---|
| 5 | 57% compile | 71% | corpus grew to 7 tasks |
| 6 | 86% | 86% | fixed what run 5 found |
| 8 | 100% | 86% | framework fix, see below |
| 9 | 100% | 71% | skill additions |
| 10 | 100% | 71% | same answers as 9, judged again |

Run 5 was the low point. With enough tasks to see anything, the skill made
things worse. The auth recipe listed `ApiToken` in a table with no example, so
the agent left out `State<AppContext>` and Axum couldn't infer the router.
Our own API index handed it a private module path. The material misled.

Run 9 looks like a win, +29 points. It isn't one. Between runs 8 and 9, nothing
the no-skill agent sees changed, byte for byte. Its score still dropped from 86%
to 71%, which means one task's worth of swing is noise. And run 10 rejudged
run 9's answers without changing a line of code, and the idiomatic score flipped
from +0.06 to -0.03. The judge's own spread, 0.09 on average, was bigger than
every difference it ever reported. So after ten runs the honest scorecard is
that the headline number never measured anything.

## What did move

The failures changed shape. Without the skill, the agent reached for Sea-ORM's
expression layer, `Expr::col(Column::Name).like(..)`, in five of six runs and
failed on a missing trait. With the skill, it wrote
`users::Column::Name.like(..)` and compiled. `errors.md` has a section that
says "reaching for `Expr::` usually means you've gone one layer too low," and
it worked where it applied.

That's a change in *which API the agent picks*, and a pass rate can't see it.
One failure is one failure.

The Rails Foundation found the same thing on a much bigger run.
[Agents on Rails](https://rubyonrails.org/2026/8/13/agents-on-rails-the-first-benchmark-report)
graded models on real Writebook tasks and tracked Rails API recall apart from solve rate. When they gave a model the Rails guides and API docs:

> Recall moved, and Luna reached for the API more and hand-rolled less. The
> success score barely did.

Documentation changes *how* an agent writes code. It barely changes *whether*
the task passes. Their benchmark had the instrument to show it. Ours, built to
count passes, was blind to it, and we spent a few runs thinking the skill didn't
work when the real problem was that we were measuring the wrong thing.

## The part we didn't expect

The eval's most useful output was neither number. It kept finding bugs in Loco
itself, and one of them we introduced while trying to raise the score.

That's [the next post](/blog/agents-5-the-eval-found-loco-bugs/).
