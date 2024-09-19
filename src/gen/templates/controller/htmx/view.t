{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: assets/views/{{file_name}}/{{action}}.html
skip_exists: true
message: "{{file_name}}/{{action}} view was added successfully."
---
<!DOCTYPE html>
<html lang="en">

<head>
    <script src="https://cdn.tailwindcss.com?plugins=forms,typography,aspect-ratio,line-clamp"></script>
</head>

<body class="prose p-10">
    <h1>View {{action}}</h1>
    Find me in <code>{{file_name}}/{{action}}</code>
</body>

</html>
