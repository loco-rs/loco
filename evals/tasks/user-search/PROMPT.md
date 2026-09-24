Add a user search endpoint to this app.

Requirements:

- `GET /api/users/search?q=<term>&page=<n>` returns users whose name or email
  contains `<term>`, case-insensitively.
- Results are paginated. The page size should be configurable rather than
  hard-coded in the handler.
- Only an authenticated caller may use it.

Do not add any new dependencies.
