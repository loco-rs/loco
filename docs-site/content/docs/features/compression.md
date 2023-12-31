+++
title = "Compression"
description = ""
date = "2023-30-12T:00:00+00:00"
updated = "2023-30-12T:00:00+00:00"
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
+++


## Compression Configuration

`Loco` leverages [CompressionLayer](https://docs.rs/tower-http/0.5.0/tower_http/compression/index.html) to enable a `one click` solution.

To enable response compression, based on `accept-encoding` request header, simply edit the configuration as follows:

```yaml
#...
  middlewares:
    etag:
      enable: true
```

Doing so will compress each response and set `content-encoding` response header accordingly.

