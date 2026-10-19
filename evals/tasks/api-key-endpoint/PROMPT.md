Machine clients integrate with this app using a long-lived API key rather than a
JWT.

Add `GET /api/machine/whoami`, which authenticates the caller by their API key
and returns their public identifier and name as JSON.

Requirements:

- Authentication is by API key, not JWT.
- An unauthenticated caller must get 401 without the handler running.
- The response must not expose anything beyond the public identifier and name.

Do not add any new dependencies.
