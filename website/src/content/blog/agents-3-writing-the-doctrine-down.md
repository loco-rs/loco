---
title: "Writing the doctrine down"
description: "Loco follows the Rails doctrine and diverges only where Rust forces it. An agent can't infer that, so we wrote it into a file. Part 3 of teaching agents Loco."
pubDate: 2026-10-06
authors:
  - team-loco
---

Templates and an API index get an agent to code that compiles. They don't get
it to code a Loco developer would merge. The gap is taste, and taste lives in
people's heads until someone writes it down.

Rails wrote it down years ago, in [The Rails Doctrine](https://rubyonrails.org/doctrine).
Convention over configuration. The menu is omakase. Integrated systems over
decoupled purity. We've always said Loco is Rails for Rust. When we sat down to
write `doctrine.md` for the skill, we had to say what that means in terms an
agent can act on:

> When you are unsure how something should work in Loco, the answer is almost
> always "the way Rails does it." Where Loco diverges, it diverges because Rust
> forced it, never because Loco disagreed with Rails.

That one rule settles more arguments than anything else in the file. No
`services/` directory, because Rails doesn't have one. No repository layer. The
model is the domain object and it knows how to save itself. That's Active
Record, on purpose.

## The seven places Rust said no

The useful part of the doctrine is the list of places where "do what Rails
does" produces code that doesn't compile. There are seven:

1. **No `method_missing`.** `User.find_by_email` doesn't appear by magic. You
   write the finder on `impl Model`. Same idea, done by hand.
2. **Ownership.** Rails mutates in place with `update!`. Loco state changes are
   methods on `ActiveModel` that consume `self` and return the saved `Model`:
   `user.into_active_model().verified(&db).await?`.
3. **No autoloading.** Rust wants `mod` declarations, route registration and
   worker registration spelled out. The generators write that wiring, which
   makes `cargo loco generate` *more* important than `rails g`.
4. **No inheritance.** `ApplicationController`'s `before_action` splits in two:
   middleware for cross-cutting concerns, extractors for per-handler ones. A
   handler that takes `auth::JWT` can't run unauthenticated. The type system
   won't let it.
5. **`Result` instead of exceptions.** Handlers return `Result<Response>` and
   use `?`. A panic is a crashed request, not a 500.
6. **Traits instead of mixins.** `Validatable` is `validates`,
   `ActiveModelBehavior` is `before_save`.
7. **Typed YAML instead of initializers.** Config is declarative and typed,
   so app code doesn't read `std::env::var`. Ever.

Every item comes with the Rails thing it replaces. An agent already knows Rails.
Hand it "this is `before_action`, and here is why it's an extractor in Loco"
and it reaches the right answer on its own.

## The question nobody had answered

Writing it down exposed a gap. Task, worker or scheduler? We had no doc that
answered it, and agents split recurring work every possible way.

We answered it from the source, not from memory. The scheduler's `Job` holds a
`run: String` and executes it as a subprocess. The scheduler holds no work. It
shells out. So recurring work in Loco is always two pieces, a `Task` that does
the thing and a schedule entry that runs it. That's `whenever` writing a crontab
that calls `rake`. Rails had the answer all along. We hadn't noticed we'd
copied it.

## Smells, ranked

The doctrine ends with ten smells in order of damage. Number one is serializing
an entity straight into a response, because `users` carries a password hash and
an API key, and the code compiles, lints and passes its tests while leaking both.
No compiler catches that. It's the kind of thing only taste catches, which is
why it's the first thing we tell an agent.

Next: [we built an eval to prove the skill works](/blog/agents-4-ten-eval-runs/).
It didn't, at first.
