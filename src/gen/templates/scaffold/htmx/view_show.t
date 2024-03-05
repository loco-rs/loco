{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: assets/views/{{file_name}}/show.html
skip_exists: true
message: "{{file_name}} view was added successfully."
---
<!DOCTYPE html>
<html lang="en">

<head>
    <script src="https://cdn.tailwindcss.com?plugins=forms,typography,aspect-ratio,line-clamp"></script>
</head>

<body class="prose p-10">
    <h1>View {{name}}: {% raw %}{{ item.id }}{% endraw %}</h1>
    <div class="mb-10">
     {% for column in columns -%}
        <div>
        <label><b>{% raw %}{{"{% endraw %}{{column.0}}{% raw %}" | capitalize }}{% endraw %}:</b> {% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}</label>
        </div>
    {% endfor -%}
    <br />
    <a href="/{{name | plural}}">Back to {{name | plural}}</a>
    </div>
</body>

</html>