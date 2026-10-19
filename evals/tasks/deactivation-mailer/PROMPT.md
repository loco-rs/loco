When an account is deactivated the user must be emailed a notice telling them
their account is closed and how long their data will be retained.

Add a mailer exposing `DeactivationMailer::send_notice(ctx, user, retention_days)`
which sends that message, with subject, HTML and plain-text bodies. The message
should address the user by name and state the retention period.

Do not add any new dependencies.
