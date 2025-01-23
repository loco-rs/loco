{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: assets/views/{{file_name}}/edit.html
skip_exists: true
message: "{{file_name}} edit view was added successfully."
---
{% raw %}{% extends "base.html" %}{% endraw %}

{% raw %}{% block title %}{% endraw %}
Edit {{name}}: {% raw %}{{ item.id }}{% endraw %}
{% raw %}{% endblock title %}{% endraw %}

{% raw %}{% block page_title %}{% endraw %}
Edit {{name}}: {% raw %}{{ item.id }}{% endraw %}
{% raw %}{% endblock page_title %}{% endraw %}

{% raw %}{% block content %}{% endraw %}
<div class="mb-10">
    <div id="error-message" class="mt-4 text-sm text-red-600"></div>
    <form hx-put="/{{name | plural}}/{% raw %}{{ item.id }}{% endraw %}" hx-ext="submitjson" hx-target="#success-message" class="flex-1 lg:max-w-2xl">
        {% for column in columns -%}
            {{ render_form_field(fname=column.0, rust_type=column.1, ftype=column.2, edit_form=true)}}
        {% endfor -%}
        <div>
            <div class="mt-5">
                <button class=" text-xs py-3 px-6 rounded-lg bg-gray-900 text-white" type="submit">Submit</button>
                <button class="text-xs py-3 px-6 rounded-lg bg-red-600 text-white"
                            onclick="confirmDelete(event, '/{{name | plural}}/{% raw %}{{ item.id }}{% endraw %}', '/{{name | plural}}' )">Delete</button>
            </div>
        </div>
    </form>
    <div id="success-message" class="mt-4"></div>
    <br />
    <a href="/{{name | plural}}">Back to {{name}}</a>
</div>
{% raw %}{% endblock content %}{% endraw %}

{% raw %}{% block js %}{% endraw %}

{% raw %}{% endblock js %}{% endraw %}