{% set module_name = name | snake_case -%}
{% set struct_name = module_name | pascal_case -%}
to: "src/data/mod.rs"
skip_exists: true
message: "Data module added"
injections:
- into: "src/lib.rs"
  append: true
  content: "pub mod data;"
---
