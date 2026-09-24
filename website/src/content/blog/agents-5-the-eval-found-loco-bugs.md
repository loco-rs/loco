---
title: "The eval kept finding bugs in Loco"
description: "We built an eval to grade agents. It graded us. What it found, the regression we shipped chasing a score, and why the generated app turned out to be the best prompt we have. Part 5 of teaching agents Loco."
pubDate: 2026-09-24T05:00:00Z
authors:
  - dotan-nahum
---

Every eval task needs a reference solution, and we wrote each one against a
real `loco new` app instead of by hand. Before a single agent had run, writing
those references had already turned up three bugs:

- **A fresh app failed `cargo clippy -- -D warnings`.** The skill tells agents  to run that before calling a change done. Every agent would have
  failed on our code before touching its own.
- **Our scheduler recipe pointed at the wrong file.** Schedules live under a
  `scheduler:` key in `config/<env>.yaml`. The standalone file we documented
  is only read when you pass `--config`.
- **Schedules accept plain English**, "every day at 3:00 am", and nothing said
  so.

## What agents tripped on was usually ours

Once agents ran, their failures kept pointing back at the framework. When a
model writes `.filter(..)` and it compiles, then `.order_by_desc(..)` on the
next line and it doesn't, the model isn't wrong. Loco's prelude exported
Sea-ORM's `QueryFilter` and not `QueryOrder`. Our own scaffold templates had
been importing it by hand to work around that, and nobody had noticed the
workaround was the bug report.

The same pattern turned up `Pager` and `PagerMeta` missing from the prelude
(agents hand-rolled a pagination envelope with the same four fields) and no
`Config::settings::<T>()` (every app deserialized custom settings by hand, and
the obvious way fell back to defaults on a typo). Both are fixed for
every Loco app, agent or human.

## The one we broke

Chasing the `Expr::like` failure from [the last post](/blog/agents-4-ten-eval-runs/),
we added Sea-ORM's `ExprTrait` to the prelude. The next run looked great. It
also broke `n.max(1)` for every Loco app, because `ExprTrait` is implemented
for anything that converts into an expression, which includes every integer.
Worse, `n.eq(&3)` kept compiling and returned an `Expr` instead of a
`bool`.

The agent's code had been wrong all along. It reached one layer too low, and
we widened the framework to accommodate the mistake. We took `ExprTrait` back
out and added tests that fail if it returns. The rule we use now is one
question: **does Loco's own generated code work around this?** It did for
`QueryOrder`. It never had for `ExprTrait`.

## The generated app is the prompt

After enough runs, the pattern was hard to miss. An agent starts from what
`loco new` hands it, and copies it. If the starter has a bug, the agent learns
the bug. If the starter has no sign-up page, the agent's app has no sign-up
page. The template teaches more than any skill file, because it's the code the
agent reads first and trusts most.

So we went through the starter the way an agent would. A server-side app
answered 404 at `/`, because the home view shipped with a test that rendered
it and no route that served it. The React starter had no sign-up page, a dev
proxy hardcoded to port 5150, booleans rendering as blank cells, and a
`vite build` that never typechecked. Scaffolded `create` and `delete` answered
`GET`. The generated model test asserted nothing. Loco 1.2.0 fixes all of it.

The deeper fix is the Rails one. Rails tests its generators as units: run the
generator, assert on the files it wrote. We do the same now. The generator
templates have tests that fail when their output regresses, and the `loco new`
matrix builds and
tests a full app for every starter combination on every PR. An agent can only
be as good as the first code it sees, so that code gets the strictest gate we
have.

## Where this leaves us

Can an agent build a good Loco app? On scoped work against a generated app,
yes: it compiles, passes clippy, and reads like code we'd merge. Most of that
comes from the model already knowing Rails, Axum and Sea-ORM. The skill moves
which APIs it reaches for. The templates decide what it copies. The compiler
tells it when it's wrong.

Conventions did the work for Rails over twenty years of public code. We don't
have the twenty years, so we write the conventions down, ship them inside every
app, and test the code agents learn from. If you build with Loco and an agent
gets something wrong, open an issue. It's often our bug.
