+++
title = "Mailers"
description = ""
date = 2021-05-01T18:20:00+00:00
updated = 2021-05-01T18:20:00+00:00
draft = false
weight = 22
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

Testing emails sent as part of your workflow can be a complex task, requiring validation of various scenarios such as email verification during user registration and checking user password emails. The primary goal is to streamline the testing process by examining the number of emails sent in the workflow, reviewing email content, and allowing for data snapshots.

In `Loco`, we have introduced a stub test email feature. Essentially, emails are not actually sent; instead, we collect information on the number of emails and their contents as part of the testing context.

To enable the stub in your tests, add the following field to the configuration under the mailer section in your YAML file:

```yaml
mailer:
  stub: true
```

Note: If your email sender operates within a [worker](@/docs/the-app/workers.md) process, ensure that the worker mode is set to ForegroundBlocking.

Once you have configured the stub, proceed to your unit tests and follow the example below:

Test Description:

- Create an HTTP request to the endpoint responsible for sending emails as part of your code.
- Retrieve the mailer instance from the context and call the deliveries() function, which contains information about the number of sent emails and their content.

```rust

#[tokio::test]
#[serial]
async fn can_register() {
    configure_insta!();

    testing::request::<App, Migrator, _, _>(|request, ctx| async move {
        // Create a request for user registration.

        // Now you can call the context mailer and use the deliveries function.
        with_settings!({
            filters => testing::cleanup_email()
        }, {
            assert_debug_snapshot!(ctx.mailer.unwrap().deliveries());
        });
    })
    .await;
}
```
