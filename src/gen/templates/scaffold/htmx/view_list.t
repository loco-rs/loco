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
</head>

<body>
     <h1>{{file_name}}s</h1>
    {% raw %}{% for item in items %}{% endraw %}
    <li>
        <a href="/api/{{name | plural}}/{% raw %}{{ item.id }}{% endraw %}">
            {% raw %}{{ item.id }}{% endraw %}
        </a>: {% raw %}{{item | json_encode()}}{% endraw %}
    </li>
    {% raw %}{% endfor %}{% endraw %}

    <br />
    <br />
    <a href="/api/{{name | plural}}/new">New {{name}}</a>
</body>

</html>