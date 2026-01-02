{% set module_name = name | snake_case -%}
{% set struct_name = module_name | pascal_case -%}
to: "src/mailers/{{module_name}}/welcome/html.t"
skip_exists: true
---
{% raw %}{% extends "base.t" %}{% endraw %}
{% raw %}{% block title %}{% endraw %}Welcome!{% raw %}{% endblock %}{% endraw %}
{% raw %}{% block body %}{% endraw %}
<h1>Welcome!</h1>
<p>welcome to <em>acmeworld!</em></p>
{% raw %}{% endblock %}{% endraw %}
