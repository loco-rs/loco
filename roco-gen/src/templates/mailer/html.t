{% set module_name = name | snake_case -%}
{% set struct_name = module_name | pascal_case -%}
to: "src/mailers/{{module_name}}/welcome/html.t"
skip_exists: true
---
welcome to <em>acmeworld!</em>
