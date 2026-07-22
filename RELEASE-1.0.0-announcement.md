# Loco 1.0.0 — announcement drafts

> Draft copy for the 1.0.0 launch, in three channels. **Jondot owns final voice
> and the personal notes.** Placeholders in `«…»`. Publish after the crates are
> live on crates.io so every link and `cargo install` works.

---

## 1) Blog post

**Title:** Loco 1.0 — the one-person framework for Rust is stable

**Slug:** `loco-1-0`

---

Today Loco reaches **1.0** — the first stable release of the Rails-inspired,
batteries-included web framework for Rust. If you've been waiting for a "you can
build your product on this" line in the sand, this is it: a single, deliberate
breaking release that sets the API we intend to stand behind.

Loco has always been about one thing — letting **one developer** move like a
whole team. 1.0 is that promise, hardened.

### The headline: Sea-ORM 2.0

The centerpiece of 1.0 is the move to **Sea-ORM 2.0** (with sqlx 0.9). For most
apps the upgrade is mechanical — Loco's `schema` helpers and the generated
model/migration shapes absorb the API churn — and the [0.16 → 1.0 upgrade
guide](https://loco.rs/docs/extras/upgrades/) walks every step.

One honest note up front: Sea-ORM 2.0 hasn't cut a *stable* release yet, so Loco
1.0 pins `sea-orm =2.0.0-rc.41` — the exact rc our full test matrix is green
against. We chose to ship 1.0 on a solid rc rather than hold the whole framework
hostage to an upstream date. The moment Sea-ORM 2.0 goes stable, we'll ship a
Loco patch that moves to it.

### Built for the age of coding agents

1.0 makes Loco a **first-class citizen for LLMs and coding agents**. Every new
app ships an `AGENTS.md` that teaches an agent how to build with Loco, and the
site serves `llms.txt` / `llms-full.txt` so tools can pull the whole framework
into context. "Claude, build me a Loco app" is now a supported workflow, not a
hope.

### What's new (a tour)

**Background jobs grew up.** Priority queues (`perform_later_with_priority`), an
opt-in reaper that rescues jobs stranded by a crashed worker, `perform_later`
now returns the job id, and the queue became a pluggable `QueueProvider` adapter.
You can also run the scheduler without a worker.

**Mailer.** Multi-recipient emails, Tera template inheritance and shared layouts,
implicit TLS (SMTPS/465), custom headers, and a synchronous `deliver_now`
alongside the enqueue-by-default `mail`.

**Web layer.** Optional JWT extraction (`Option<JWT>` to serve authed *and*
anonymous callers from one handler), verb-explicit route builders
(`Routes::new().get(...)`), and `MiddlewareStackExt` for surgical edits to the
default middleware stack (`insert_before` / `replace` / `delete`, Rails-style).

**Ops.** TLS to managed Postgres and Redis (no C toolchain), and a typed,
streaming `db::dump` that round-trips your data with full type fidelity.

### A deep hardening pass

1.0 isn't just features. We did a security- and correctness-focused sweep across
the queue, storage, config, error, remote-IP, and middleware subsystems. A few
that matter:

- **Local storage no longer roots at `/`** — a user-controlled key could
  previously escape to the whole disk. It now roots at the working directory.
- **Error → HTTP status mapping is honest and exhaustive** — `EntityNotFound`
  returns `404`, `EntityAlreadyExists` returns `409`, validation errors return
  `4xx`, and adding a new error variant is now a compile error until you classify
  it (no more silent `500`s).
- **`remote_ip` was rebuilt on `axum-client-ip`** with an explicit trust model.
  If you run behind multiple proxies, read that section of the upgrade guide — the
  old `trusted_proxies` behavior changed.
- **JWT is restricted to the HMAC family**, so asymmetric algorithms that could
  never work with Loco's shared secret can't be misconfigured into broken tokens.

There's much more — 64-bit primary keys by default, `{env}.local.yaml` config
deep-merging, edition 2024 internals, and a broad dependency modernization. The
full detail is in the [CHANGELOG](https://github.com/loco-rs/loco/blob/master/CHANGELOG.md).

### Get started

```sh
cargo install loco
cargo install sea-orm-cli --version '=2.0.0-rc.41'
loco new
```

Then open [loco.rs](https://loco.rs) and the [docs](https://loco.rs/docs/).

### Thank you

«Personal note from Jondot here.» 1.0 carries work and reports from a lot of
people — huge thanks to @elcoosp, @jtwaleson, @NewtTheWolf, @mccormickt, @zmilan,
@YtimoDeng, @dsgallups, @GoCoder7, @D-system, @zjom, @lunfel, @labike, @floscodes,
@Leandros, @askor, @RandomInsano, @Nam-T, and everyone who filed an issue or
tested an rc. And to @kaplanelad for co-maintaining the whole way.

Build something today.

— The Loco team

---

## 2) Discord

> **🚂 Loco 1.0 is out — Loco is stable.**
> The one-person framework for Rust hits 1.0: Sea-ORM 2.0, first-class support
> for coding agents (`AGENTS.md` + `llms.txt`), priority queues, TLS to
> Postgres/Redis, a big security/hardening pass, and a clean 0.16 → 1.0 upgrade
> guide.
>
> Upgrade guide → https://loco.rs/docs/extras/upgrades/
> Changelog → https://github.com/loco-rs/loco/blob/master/CHANGELOG.md
>
> `cargo install loco && loco new`
>
> Heads-up: 1.0 pins `sea-orm =2.0.0-rc.41` (we ship on the rc rather than wait
> for stable; a patch will move to stable when it lands). Thank you all for
> getting us here. 🙏

---

## 3) X / Twitter thread

**1/**
Loco 1.0 is out. 🚂

The one-person framework for Rust — Rails-inspired, batteries included — is now
stable.

cargo install loco && loco new

🧵 what's in it:

**2/**
The headline: **Sea-ORM 2.0**.

For most apps the upgrade is mechanical — our schema helpers absorb the churn.
Full step-by-step 0.16 → 1.0 guide: https://loco.rs/docs/extras/upgrades/

**3/**
Loco is now built for **coding agents**.

Every new app ships an AGENTS.md that teaches an agent to build with Loco, and we
serve llms.txt / llms-full.txt. "Build me a Loco app" is a first-class workflow.

**4/**
New in 1.0: priority queues + a job reaper, multi-recipient email w/ template
inheritance, implicit SMTP TLS, run the scheduler without a worker, optional JWT
extraction, verb-explicit routes, TLS to managed Postgres & Redis, typed
streaming db::dump.

**5/**
And a real security + correctness pass: local storage no longer roots at `/`,
honest 4xx/5xx error mapping, a rebuilt remote_ip trust model, HMAC-only JWT.

**6/**
Honest note: 1.0 pins sea-orm =2.0.0-rc.41. We shipped on the rc our test matrix
is green against rather than hold 1.0 for an upstream stable date. A patch moves
to stable when it lands.

**7/**
Thank you to everyone who filed issues, sent PRs, and tested the rcs. 1.0 is
yours as much as ours.

Docs: https://loco.rs
Changelog: https://github.com/loco-rs/loco/blob/master/CHANGELOG.md

Build something today.
