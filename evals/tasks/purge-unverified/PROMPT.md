When an administrator triggers a cleanup, this app must delete every user who
registered more than 30 days ago and never verified their email address.

Requirements:

- The work must survive a process restart and must not block the HTTP request
  that triggers it.
- The retention window (30 days) is passed in when the work is enqueued.
- Log how many users were removed.

Do not add any new dependencies.
