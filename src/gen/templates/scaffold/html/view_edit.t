{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: assets/views/{{file_name}}/edit.html
skip_exists: true
message: "{{file_name}} edit view was added successfully."
---
<!DOCTYPE html>
<html lang="en">

<head>
    <script src="https://cdn.tailwindcss.com?plugins=forms,typography,aspect-ratio,line-clamp"></script>
    <script>
        function confirmDelete(event) {
            event.preventDefault();
            if (confirm("Are you sure you want to delete this item?")) {
                var xhr = new XMLHttpRequest();
                xhr.open("DELETE", "/{{name | plural}}/{% raw %}{{ item.id }}{% endraw %}", true);
                xhr.onreadystatechange = function () {
                    if (xhr.readyState == 4 && xhr.status == 200) {
                        window.location.href = "/{{name | plural}}";
                    }
                };
                xhr.send();
            }
        }
    </script>
</head>

<body class="prose p-10">
    <h1>Edit {{name}}: {% raw %}{{ item.id }}{% endraw %}</h1>
    <div class="mb-10">
    <form action="/{{name | plural}}/{% raw %}{{ item.id }}{% endraw %}" method="post">
    <div class="mb-5">
     {% for column in columns -%}
        <div>
        <label>{{column.0}}</label>
        <br />
        {% if column.2 == "text" -%}
        <textarea id="{{column.0}}" name="{{column.0}}" type="text">{% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}</textarea>
        {% elif column.2 == "string" -%}
        <input id="{{column.0}}" name="{{column.0}}" type="text" value="{% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}"></input>
        {% elif column.2 == "string!" or column.2 == "string^" -%}
        <input id="{{column.0}}" name="{{column.0}}" type="text" value="{% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}" required></input>
        {% elif column.2 == "int" or column.2 == "int!" or column.2 == "int^"-%}
        <input id="{{column.0}}" name="{{column.0}}" type="number" required value="{% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}"></input>
        {% elif column.2 == "bool"-%}
        <input id="{{column.0}}" name="{{column.0}}" type="checkbox" value="true" {% raw %}{% if item.publish %}checked{%endif %}{% endraw %}></input>
        {% elif column.2 == "bool!"-%}
        <input id="{{column.0}}" name="{{column.0}}" type="checkbox" value="true" required {% raw %}{% if item.publish %}checked{%endif %}{% endraw %}></input>
        {% elif column.2 == "ts"-%}
        <input id="{{column.0}}" name="{{column.0}}" type="text" value="{% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}"></input>
        {% elif column.2 == "ts!"-%}
        <input id="{{column.0}}" name="{{column.0}}" type="text" value="{% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}" required></input>
        {% elif column.2 == "uuid"-%}
        <input id="{{column.0}}" name="{{column.0}}" type="text" value="{% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}"></input>
        {% elif column.2 == "uuid!"-%}
        <input id="{{column.0}}" name="{{column.0}}" type="text" value="{% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}" required></input>
        {% elif column.2 == "json" or column.2 == "jsonb" -%}
        <textarea id="{{column.0}}" name="{{column.0}}" type="text">{% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}</textarea>
        {% elif column.2 == "json!" or column.2 == "jsonb!" -%}
        <textarea id="{{column.0}}" name="{{column.0}}" type="text" required>{% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}</textarea>
        {% endif -%} 
        </div>        
    {% endfor -%}
    <div>
    <div class="mt-5">
            <button class=" text-xs py-3 px-6 rounded-lg bg-gray-900 text-white" type="submit">Submit</button>
            <button class="text-xs py-3 px-6 rounded-lg bg-red-600 text-white"
                        onclick="confirmDelete(event)">Delete</button>
        </div>
    </form>
    <a href="/{{name | plural}}">Back to {{name | plural}}</a>
    </div>
</body>

</html>
