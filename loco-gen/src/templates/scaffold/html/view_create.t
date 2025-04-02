{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: assets/views/{{file_name}}/create.html
skip_exists: true
message: "{{file_name}} create view was added successfully."
---
{% raw %}{% extends "base.html" %}{% endraw %}

{% raw %}{% block title %}{% endraw %}
Create {{file_name}}
{% raw %}{% endblock title %}{% endraw %}

{% raw %}{% block page_title %}{% endraw %}
Create new {{name}}
{% raw %}{% endblock page_title %}{% endraw %}

{% raw %}{% block content %}{% endraw %}
<div class="mb-10">
    <form action="/{{name | plural}}" method="post" class="flex-1 lg:max-w-2xl">
    {% for column in columns -%}
            {{ render_form_field(fname=column.0, rust_type=column.1, ftype=column.2)}}
        {% endfor -%}
        <div class="mt-5">
            <button class=" text-xs py-3 px-6 rounded-lg bg-gray-900 text-white" type="submit">Submit</button>
        </div>
    </form>
<br />
<a href="/{{name | plural}}">Back to {{name | plural}}</a>
</div>
{% raw %}{% endblock content %}{% endraw %}

{% raw %}{% block js %}{% endraw %}

{% raw %}{% endblock js %}{% endraw %}