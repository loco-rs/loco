{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: assets/views/{{file_name}}/list.html
skip_exists: true
message: "{{file_name}} list view was added successfully."
---
<!DOCTYPE html>
<html lang="en">

<head>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <script src="https://cdn.tailwindcss.com?plugins=forms,typography,aspect-ratio,line-clamp"></script>
</head>

<body class="prose p-10">
     <h1>{{file_name}}s</h1>
     <div class="mb-10">
    {% raw %}{% for item in items %}{% endraw %}
    <div class="mb-5">
            {% for column in columns -%}
                <div>
                <label><b>{% raw %}{{"{% endraw %}{{column.0}}{% raw %}" | capitalize }}{% endraw %}:</b> {% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}</label>
                </div>
            {% endfor -%}
            <a href="/{{name | plural}}/{% raw %}{{ item.id }}{% endraw %}/edit">Edit</a>
            <a href="/{{name | plural}}/{% raw %}{{ item.id }}{% endraw %}">View</a>
        </div>
    {% raw %}{% endfor %}{% endraw %}

    <br />
    <br />
    <a href="/{{name | plural}}/new">New {{name}}</a>
    </div>
</body>

</html>