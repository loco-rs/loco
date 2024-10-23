{% set module_name = name | snake_case -%}
{% set struct_name = module_name | pascal_case -%}
to: "src/mailers/{{module_name}}/welcome/subject.t"
skip_exists: true
---
guess what? welcome!
