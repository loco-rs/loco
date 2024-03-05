{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: assets/views/{{file_name}}/edit.html
skip_exists: true
message: "{{file_name}} edit view was added successfully."
---
<!DOCTYPE html>
<html lang="en">

<head>
</head>

<body>
    <h1>Edit {{name}}: {% raw %}{{ item.id }}{% endraw %}</h1>
    <form action="/api/{{name | plural}}/{% raw %}{{ item.id }}{% endraw %}" method="post">
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
            <button type="submit">Submit</button>
        </div>
    </form>
    <br />
    <a href="/api/{{name | plural}}">Back to {{name}}</a>
</body>

</html>