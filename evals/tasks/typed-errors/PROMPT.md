Add `POST /api/verification/{pid}`, which marks a user's email address as
verified.

It must answer with the right HTTP status in each case:

- no user with that public id — 404
- the user has already been verified — 400, with a message saying so
- otherwise mark them verified and return 200 with an empty JSON body

Do not add any new dependencies.
