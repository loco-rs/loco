Add `GET /api/stats/users`, returning the total number of registered users as
JSON.

Counting is expensive and the number does not need to be exact, so the result
should be cached for 60 seconds rather than recomputed on every request. The
cache must be shared across the whole application.

Do not add any new dependencies.
