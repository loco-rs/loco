{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: assets/views/{{file_name}}/show.html
skip_exists: true
message: "{{file_name}} view was added successfully."
---
<!DOCTYPE html>
<html lang="en">

<head>
</head>

<body>
    <h1>View {{name}}: {% raw %}{{ item.id }}{% endraw %}</h1>
     {% for column in columns -%}
        <div>
        <label>{{column.0}}: {% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}</label>
        </div>
    {% endfor -%}
    <br />
    <a href="/api/{{name | plural}}">Back to {{name}}</a>
</body>

</html>