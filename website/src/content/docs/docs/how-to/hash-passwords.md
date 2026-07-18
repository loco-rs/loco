---
title: Hash and Verify Passwords
description: "Use loco_rs::hash to Argon2id-hash passwords, verify them on login, and generate random tokens for reset links or API keys."
sidebar:
  order: 43
---

Goal: store passwords safely at registration, verify them at login, and generate random strings for things like reset tokens or API keys.

`loco_rs::hash` wraps Argon2id (`argon2` crate) for password hashing and a random alphanumeric generator for tokens. Unlike the JWT auth machinery, this module is **not** feature-gated — it's always available, with no Cargo feature to enable.

## Hash a password on registration

```rust
use loco_rs::hash;

let hashed = hash::hash_password("the-users-plaintext-password")?;
// store `hashed` in your user row, never the plaintext
```

`hash_password` uses Argon2id with a fresh random salt (`OsRng`) per call, so hashing the same password twice produces two different strings — that's expected; `verify_password` (below) handles the comparison. It returns Loco's `Result<String>`; a hashing failure surfaces as `Error::Message` via `Error::msg`.

## Verify a password on login

```rust
use loco_rs::hash;

if hash::verify_password(&submitted_password, &user.password_hash) {
    // credentials are valid
} else {
    // reject the login
}
```

`verify_password` returns a plain `bool`, not a `Result` — it returns `false` for both "wrong password" and "malformed/foreign hash string", so a bad row in the database fails closed rather than raising an error you'd have to remember to handle. It's `#[must_use]`, so the compiler will warn if you ignore the result.

## Generate random tokens

```rust
use loco_rs::hash;

let reset_token = hash::random_string(32);
let api_key = hash::random_string(40);
```

`random_string(length)` returns an alphanumeric string of exactly `length` characters, suitable for password-reset tokens or per-user API keys — see [Protect a route with an API key](@/docs/how-to/api-key-auth.md) for wiring a generated key into `Authenticable::find_by_api_key`.

## Verify it works

A quick round-trip in a test or a scratch binary:

```rust
let pass = "correct horse battery staple";
let hashed = hash::hash_password(pass)?;

assert!(hash::verify_password(pass, &hashed));
assert!(!hash::verify_password("wrong password", &hashed));
```

## Related

- [Protect a route with an API key](@/docs/how-to/api-key-auth.md) — use `random_string` to mint the key, `find_by_api_key` to look it up.
- [Protect a route with JWT](@/docs/how-to/jwt-auth.md) — issue a token once `verify_password` confirms the login.
- [Error model reference](@/docs/reference/errors.md) — how `Error::msg`/`Error::Message` fit into the crate's error type.
