{% set module_name = name | snake_case -%}
{% set struct_name = module_name | pascal_case -%}
to: "src/mailers/shared/text.t"
skip_exists: true
---
{% raw %}{% block text %}{% endraw %}{% raw %}{% endblock %}{% endraw %}

