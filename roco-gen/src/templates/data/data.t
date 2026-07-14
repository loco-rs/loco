{% set module_name = name | snake_case -%}
to: "data/{{module_name}}/data.json"
skip_exists: true
---
{ 
  "is_loaded": true
}
