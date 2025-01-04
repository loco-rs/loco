+++
title = "Session Store"
description = ""
date = 2025-01-01T00:00:00+00:00
updated = 2025-01-01T00:00:00+00:00
draft = false
weight = 5
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++



Session in `Loco` provided the functionality to store data that will be persisted between requests. By configuration, the session can be backed by Cookie based store or the tower-based approach.


## Overview

The request context middleware allows you to handle session data for each incoming request. By configuring it properly, you can choose between a cookie-based store (default, development) and other session providers (e.g., the tower-based approach). It provides flexibility to switch providers based on your application's needs.


## Prerequisites
`LocoRequestId` middleware is required (included within the default middleware stack by default). This middleware provides a unique request ID for each request, which can be used to track requests across multiple sessions. 

- Debugging: There will be an error message `missing request_id request extension` with debug span named `RequestContextService::call` if the `LocoRequestId` middleware is not present.


## Supported Session Providers

1. **Cookie-based Session (Development Use)**
   Stores encrypted and signed session data in cookies using `PrivateCookieJar`.  
   • Suitable for lightweight data (under 4KB).  
   • Requires a secure, random private key of at least 64 bytes.  
   • Typically recommended for production when combined with TLS/SSL.  

2. **Tower-based Session**  
   Uses an internal session store (e.g., in-memory or an alternative backend).  
   • Great for scenarios where you want the server to manage data centrally.  
   • You must provide a session store instance to the middleware.


## Configuration

### Cookie Session
Below is a sample configuration you can place in a YAML file (e.g., `config/development.yaml`). This config shows how to enable the request context middleware with a cookie-based session:

```yaml
# config/development.yaml
request_context:
  enable: true
  session_config:
    name: "__loco_session"
    http_only: true
    same_site:
      type: Lax
    expiry: 3600         # in seconds
    secure: false        # set true for production
    path: "/"
    # domain: "example.com"  # optional, if you need a specific domain
  session_store:
    type: Cookie
    value:
      private_key: <YOUR_PRIVATE_KEY_AT_LEAST_64_BYTES>
```

Replace `<YOUR_PRIVATE_KEY_AT_LEAST_64_BYTES>` with your actual secret. For stronger security, generate a random 64+ byte key (for instance, by using a cryptographically secure random generator).

### Tower Session
If you want to use the tower-based session approach, you could change the configuration to:

```yaml
# config/development.yaml
request_context:
  enable: true
  session_config:
    name: "__loco_session"
    http_only: true
    same_site:
      type: Strict
    expiry: 3600
    secure: true
    path: "/"
  session_store:
    type: Tower
```

In this case, ensure you have a session store included in your application. The tower-based approach can allow in-memory storage, or you can adapt it to use a different backend (database, cache, etc.).

```rust
// src/app.rs
use loco_rs::{
   app::{AppContext, Hooks},
   prelude::*,
};
use loco_rs::request_context::TowerSessionStore;

#[async_trait]
impl Hooks for App {
    // ...
  async fn after_context(ctx: AppContext) -> Result<AppContext> {
        Ok(AppContext {
           // Set up the tower session backend using [`TowerSessionStore`] Wrapper
            session_store: Some(TowerSessionStore::new(MemoryStore::default())),
            ..ctx
        })
    } 
}
```

## Usage

`RequestContext` can directly extracted within handlers:

```rust
// src/controllers/<your_controller>.rs
use loco_rs::prelude::*;

pub async fn your_handler(req: RequestContext) -> Result<Response> {
    // ...
}
```

Insert value by key, the value with a type `T` can be any type with `serde::Serialize + Send + Sync`.

```rust
// src/controllers/<your_controller>.rs
use loco_rs::prelude::*;
const REQUEST_CONTEXT_DATA_KEY: &str = "alan";

pub async fn your_handler(req: RequestContext) -> Result<Response> {
    // ...
    req.insert(REQUEST_CONTEXT_DATA_KEY, "turing").await?;
    // ...
}
```

Get data by key - the value is `Option<String>` and can be `None` if the key is not present.
```rust
// src/controllers/<your_controller>.rs
use loco_rs::prelude::*;
const REQUEST_CONTEXT_DATA_KEY: &str = "alan";

pub async fn your_handler(req: RequestContext) -> Result<Response> {
    // ...
   let data = req
           .get::<String>(REQUEST_CONTEXT_DATA_KEY)
           .await?
           .unwrap_or_default();    
   // ...
}
```

Remove data by key - the value is `Option<String>` and can be `None` if the key is not present.
```rust
// src/controllers/<your_controller>.rs
use loco_rs::prelude::*;
const REQUEST_CONTEXT_DATA_KEY: &str = "alan";

pub async fn your_handler(req: RequestContext) -> Result<Response> {
    // ...
   let data = req
           .remove(REQUEST_CONTEXT_DATA_KEY)
           .await?
           .unwrap_or_default();    
   // ...
}
```

Clear the session - Returns `()` if successful 

Tower - Clears the session but not the session store. 

Cookie - Clear the session map.
```rust
// src/controllers/<your_controller>.rs
use loco_rs::prelude::*;

pub async fn your_handler(req: RequestContext) -> Result<Response> {
    // ...
    req.clear().await?;
    // ...
}
```

Flush the session store - Returns `()` if successful

Tower - Flush the session store.

Cookie - Flush the session map.
```rust
// src/controllers/<your_controller>.rs
use loco_rs::prelude::*;

pub async fn your_handler(req: RequestContext) -> Result<Response> {
    // ...
    req.flush().await?;
    // ...
}
```

## Testing

In order to persist sessions between requests, cloning the cookies into the new request header is required.

Here is a simple example:

```rust
use serial_test::serial;
use loco_rs::testing::prelude::*;
use axum::http::HeaderName;

#[tokio::test]
#[serial]
async fn get_request_context_with_setting_data() {
    configure_insta!();
    request::<App, _, _>(|request, _ctx| async move {
        // Storing data into the session
        let response = request.post("/session").await;
        // Extracting Cookie which contains the session from response header
        let headers = response.headers();
        let cookie_value = headers.get("set-cookie");
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.text(), "turing");
        assert!(cookie_value.is_some());
        let data = response.text();
        
       // Storing cookie into the new request
        let response = request
            .get("/session")
            .add_header(
                "cookie".parse::<HeaderName>().unwrap(),
                cookie_value.unwrap().clone(),
            )
            .await;
       
        // Get response body
        assert_eq!(response.status_code(), 200);
        assert_eq!(response.text(), data);
    })
    .await;
}
```