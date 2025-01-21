to: "Shuttle.toml"
skip_exists: true
message: "Shuttle.toml file created successfully"
---
[deploy]
include = [
    "config/production.yaml"
]

[build]
assets = [
    "config/production.yaml"
]
