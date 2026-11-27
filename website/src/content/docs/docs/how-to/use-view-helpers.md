---
title: Use view helpers
description: "Format times, counts, lists and numbers the way Rails does: time_ago_in_words, pluralize, to_sentence, ordinalize, number_with_delimiter and number_to_human, as Tera filters and as Rust functions."
sidebar:
  order: 12
---

**Goal:** write "3 days ago", "2 comments", "Ana, Bo, and Cy" or "1.2 Million" in a page or an email without formatting it by hand.

Loco ships the Rails text helpers. Each one is a Tera filter, available in every template rendered through `ViewEngine` or `format::template`, and a plain Rust function in `loco_rs::controller::views::helpers` for mailers, JSON responses and tests. Both produce the same words as Rails.

| Rails | Tera filter | Rust function |
|---|---|---|
| `time_ago_in_words(t)` | `t \| time_ago_in_words` | `helpers::time_ago_in_words(t, false)` |
| `distance_of_time_in_words(a, b)` | `a \| distance_of_time_in_words(to=b)` | `helpers::distance_of_time_in_words(a, b, false)` |
| `pluralize(n, "person")` | `n \| pluralize(word="person")` | `helpers::pluralize(n, "person", None)` |
| `names.to_sentence` | `names \| to_sentence` | `helpers::to_sentence(&names)` |
| `22.ordinalize` | `22 \| ordinalize` | `helpers::ordinalize(22)` |
| `number_with_delimiter(n)` | `n \| number_with_delimiter` | `helpers::number_with_delimiter(n)` |
| `number_to_human(n)` | `n \| number_to_human` | `helpers::number_to_human(n)` |

## 1. Times

```jinja
Posted {{ post.created_at | time_ago_in_words }} ago
Took {{ job.started_at | distance_of_time_in_words(to=job.finished_at) }}
```

renders `Posted 3 days ago` and `Took about 2 hours`. Like Rails, the helper does not add "ago" — the template does, so the same filter works for "in 3 days".

The value can be anything a model serializes a time to: an RFC 3339 string (`DateTime<Utc>`, `DateTimeWithTimeZone`), a naive date-time string (read as UTC), a date (midnight UTC), or an integer of Unix seconds. Anything else is a render error that names the filter.

Keyword arguments:

- `include_seconds=true` — finer wording under a minute (`less than 5 seconds`, `half a minute`), as Rails' `include_seconds: true`.
- `now=` (on `time_ago_in_words`) — measure from a fixed time instead of the clock. Use it in tests, and in emails rendered ahead of time.

In Rust, both functions take anything that converts into `DateTime<Utc>`:

```rust
use chrono::{Duration, Utc};
use loco_rs::controller::views::helpers;

let now = Utc::now();
assert_eq!(
    helpers::distance_of_time_in_words(now - Duration::minutes(50), now, false),
    "about 1 hour"
);
```

## 2. Counts and lists

```jinja
{{ comments | length | pluralize(word="comment") }}
{{ people | pluralize(word="person") }}
{{ count | pluralize(word="octopus", plural="octopi") }}
{{ authors | to_sentence }}
{{ choices | to_sentence(last_word_connector=" or ") }}
```

`pluralize` writes the count and the word, using the inflector for the plural (`person` → `people`) unless `plural=` overrides it. The word stays singular only for a count of exactly 1, as in Rails, so `0 comments` and `1.5 comments`.

:::note
Tera has its own `pluralize` filter that returns only a suffix (`comment{{ n | pluralize }}`). Loco's filter replaces it but keeps that behaviour when `word=` is not given, so existing templates render as before.
:::

`to_sentence` joins an array with `, ` and `, and ` (Rails' defaults, Oxford comma included). Override with `words_connector`, `two_words_connector` and `last_word_connector`. In Rust, use `helpers::to_sentence_with(&items, &SentenceConnectors { .. })`.

`ordinalize` turns `1`, `22`, `113` into `1st`, `22nd`, `113th`. `helpers::ordinal(n)` returns only the suffix.

## 3. Numbers

```jinja
{{ total | number_with_delimiter }}                         {# 1,234,567.89 #}
{{ total | number_with_delimiter(delimiter=".", separator=",") }} {# 1.234.567,89 #}
{{ views | number_to_human }}                               {# 1.23 Million #}
{{ views | number_to_human(precision=2) }}                  {# 1.2 Million #}
```

`number_to_human` rounds to `precision` significant digits (default 3), drops trailing zeros, and names the unit from Thousand up to Quadrillion. Pass `significant=false` to count `precision` as decimal places instead. Like Rails, it rounds before choosing the unit, so `999999` is `1 Million`.

## Verify

Render a template with a fixed `now`, so the test does not depend on the clock:

```rust
use loco_rs::prelude::*;
use serde_json::json;

let page = format::template(
    "{{ at | time_ago_in_words(now=\"2024-06-04T12:00:00Z\") }} ago",
    json!({ "at": "2024-06-01T12:00:00Z" }),
)?;
// body: "3 days ago"
```

## Next

- [Render server-side views](/docs/how-to/render-views)
- [Send email](/docs/how-to/send-email) — the Rust functions work in mailer data too
