{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: assets/views/{{file_name}}/show.html
skip_exists: true
message: "{{file_name}} view was added successfully."
---
{% raw %}{% extends "base.html" %}{% endraw %}

{% raw %}{% block title %}{% endraw %}
View {{name}}: {% raw %}{{ item.id }}{% endraw %}
{% raw %}{% endblock title %}{% endraw %}

{% raw %}{% block page_title %}{% endraw %}
View {{name}}: {% raw %}{{ item.id }}{% endraw %}
{% raw %}{% endblock page_title %}{% endraw %}


{% raw %}{% block content %}{% endraw %}
<div class="mb-10">
    {% for column in columns -%}
    <div>
    <label><b>{% raw %}{{"{% endraw %}{{column.0}}{% raw %}" | capitalize }}{% endraw %}:</b> {% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}</label>
    </div>
{% endfor -%}
<br />
<a href="/{{name | plural}}">Back to {{name | plural}}</a>
</div>
{% raw %}{% endblock content %}{% endraw %}
