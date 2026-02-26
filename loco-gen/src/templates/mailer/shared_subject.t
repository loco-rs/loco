{% set module_name = name | snake_case -%}
{% set struct_name = module_name | pascal_case -%}
to: "src/mailers/shared/subject.t"
skip_exists: true
---
{% raw %}{% block subject %}{% endraw %}Email{% raw %}{% endblock %}{% endraw %}

