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

{% raw %}{% block content %}{% endraw %}
<h1>Create new {{file_name}}</h1>
<div class="mb-10">
    <form action="/{{name | plural}}" method="post">
        <div class="mb-5">
        {% for column in columns -%}
        <div>
            <label>{{column.0}}</label>
            <br />
            {% if column.2 == "text" -%}
            <textarea id="{{column.0}}" name="{{column.0}}" type="text" value="" rows="10" cols="50"></textarea>
            {% elif column.2 == "string" -%}
            <input id="{{column.0}}" name="{{column.0}}" type="text" value=""/>
            {% elif column.2 == "string!" or column.2 == "string^" -%}
            <input id="{{column.0}}" name="{{column.0}}" type="text" value="" required/>
            {% elif column.2 == "int" or column.2 == "int!" or column.2 == "int^"-%}
            <input id="{{column.0}}" name="{{column.0}}" type="number" required></input>
            {% elif column.2 == "bool"-%}
            <input id="{{column.0}}" name="{{column.0}}" type="checkbox" value="true"/>
            {% elif column.2 == "bool!"-%}
            <input id="{{column.0}}" name="{{column.0}}" type="checkbox" value="true" required/>
            {% elif column.2 == "ts"-%}
            <input id="{{column.0}}" name="{{column.0}}" type="text" value=""/>
            {% elif column.2 == "ts!"-%}
            <input id="{{column.0}}" name="{{column.0}}" type="text" value="" required/>
            {% elif column.2 == "uuid"-%}
            <input id="{{column.0}}" name="{{column.0}}" type="text" value=""/>
            {% elif column.2 == "uuid!"-%}
            <input id="{{column.0}}" name="{{column.0}}" type="text" value="" required/>
            {% elif column.2 == "json" or column.2 == "jsonb" -%}
            <textarea id="{{column.0}}" name="{{column.0}}" type="text" value="" rows="10" cols="50"></textarea/>
            {% elif column.2 == "json!" or column.2 == "jsonb!" -%}
            <textarea id="{{column.0}}" name="{{column.0}}" type="text" value="" required rows="10" cols="50"></textarea>
            {% elif column.2 == "array!" or column.2 == "array^" -%}
                <div id="{{column.0}}-inputs"> 
                    <input name="{{column.0}}[]" type="text" class="mb-2" required />
                </div>
                <button type="button" class="text-xs py-1 px-3 rounded-lg bg-gray-900 text-white add-more" data-group="{{column.0}}">Add More</button>
            {% elif column.2 == "array"  -%}
                <div id="{{column.0}}-inputs">
                    <input name="{{column.0}}[]" class="mb-2" type="text" />
                </div>
                <button type="button" class="text-xs py-1 px-3 rounded-lg bg-gray-900 text-white add-more" data-group="{{column.0}}">Add More</button>
            {% endif -%} 
        </div>
        {% endfor -%}
        </div>
        <div>
            <button class=" text-xs py-3 px-6 rounded-lg bg-gray-900 text-white" type="submit">Submit</button>
        </div>
    </form>
<br />
<a href="/{{name | plural}}">Back to {{name | plural}}</a>
</div>
{% raw %}{% endblock content %}{% endraw %}

{% raw %}{% block js %}{% endraw %}
<script>
    document.addEventListener('DOMContentLoaded', function () {
        document.querySelectorAll('.add-more').forEach(button => {
            button.addEventListener('click', function () {
                const group = this.getAttribute('data-group');
                const container = document.getElementById(`${group}-inputs`);
                const newInput = document.createElement('input');
                newInput.type = 'text';
                newInput.name = `${group}[]`;
                newInput.placeholder = `Enter another ${group} value`;
                container.appendChild(newInput);
            });
        });
    });
</script>
{% raw %}{% endblock js %}{% endraw %}