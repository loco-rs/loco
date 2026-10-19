---
title: "Rails had twenty years of training data. Loco has a skill."
description: "Coding agents are good at Rails because Rails has two decades of public code behind it. Loco has none of that. Part 1 of how we taught agents to write Loco."
pubDate: 2026-09-24T01:00:00Z
authors:
  - dotan-nahum
---

DHH [put it in one line](https://x.com/dhh/status/2018574874675929544)
earlier this year:

> Convention over configuration set the path for 20+ years of great training
> data for AI to use today.

He's right, and it's the best argument for conventions anyone has made in a
while. An agent writing a Rails app doesn't have to guess where the mailer goes
or what a finder is called. It has read a hundred thousand Rails apps that all
agree.

Loco is Rails for Rust. We took the conventions, the generators, fat models,
the omakase menu, and rebuilt them where Rust would let us. What we can't copy
is the twenty years. No frontier model has trained on a meaningful amount of
Loco code, and none will on any timeline we can plan around. We are not Vercel
with Next.js.

So we had a plain question: point Claude Code at an empty directory, say "build
me an app with Loco," and does it work? Does the result look like something a
Loco developer would write, or like Axum with extra steps?

## What an agent does without help

It does better than you'd expect, and worse than you'd accept.

Loco is Rails plus Axum plus Sea-ORM, and a model knows all three. It gets most
of the way on priors. Then it guesses. It invents `loco_rs::app::AppState`,
which sounds right and doesn't exist. It adds a crate for retries when Loco
ships a queue. It writes `Expr::col(Column::Name).like(..)` one layer too low
and fights the compiler for ten turns.

And when it gets stuck, it does the reasonable thing. It opens the framework
source and starts reading. All of it. That works in the end, and it's the most
expensive way to learn anything. Worse, framework source answers "how is Loco
built," which is a different question from "how should my app be written."
Nothing in 39,000 lines of `src/` says fat model, slim controller.

## What we built

Over the last month we built three things, and this series is about all of
them, including the parts that didn't work:

1. **A Loco skill.** Every app `loco new` generates now carries
   `.claude/skills/loco/`: a short router, the doctrine, ten task recipes, an
   errors guide, and API indexes for `loco-rs` and Sea-ORM generated from
   rustdoc. It follows the open Agent Skills format, so Claude, Codex, Cursor
   and friends all read it.
2. **An eval.** Ten logged runs of agents writing Loco with and without the
   skill, including the run where the skill made things worse.
3. **A pile of framework fixes** the eval dug up, most of which help every Loco
   app whether an agent wrote it or not.

The short version of where we ended up: conventions still do the heavy lifting,
the same way they do for Rails. The difference is that we can't count on a
model having absorbed ours, so we write them down and ship them inside the app.
And Rust hands us something Rails never had, a compiler that tells the agent
it's wrong before anyone reviews a line.

Next: [what is in the skill](/blog/agents-2-whats-in-the-skill/), and
why `llms.txt` wasn't the answer.
