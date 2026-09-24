Every night at 3:00 AM, this app should report how many users signed up in the
previous 24 hours.

Requirements:

- The count comes from the `users` table, using `created_at`.
- An operator must also be able to run the same report by hand, at any time,
  from the command line — without waiting for 3 AM and without editing code.
- The number of hours to look back should be adjustable when running it by hand,
  defaulting to 24.
- Log the result.

Implement this in the app. Do not add any new dependencies.
