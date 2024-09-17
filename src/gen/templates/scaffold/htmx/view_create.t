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
<div class="mb-10">
    <form hx-post="/{{name | plural}}" hx-ext="submitjson">
        <h1>Create new {{name}}</h1>
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
        {% endif -%} 
        </div>
    {% endfor -%}
    </div>
    <div>
        <button class=" text-xs py-3 px-6 rounded-lg bg-gray-900 text-white" type="submit">Submit</button>
    </div>
    </form>
</div>
{% raw %}{% endblock content %}{% endraw %}

{% raw %}{% block js %}{% endraw %}
<script>
    htmx.defineExtension('submitjson', {
        onEvent: function (name, evt) {
            if (name === "htmx:configRequest") {
                evt.detail.headers['Content-Type'] = "application/json"
            }
        },
        encodeParameters: function (xhr, parameters, elt) {
            const json = {};
            for (const [key, value] of Object.entries(parameters)) {
                const inputType = elt.querySelector(`[name=${key}]`).type;
                if (inputType === 'number') {
                    json[key] = parseFloat(value);
                } else if (inputType === 'checkbox') {
                    json[key] = elt.querySelector(`[name=${key}]`).checked;
                } else {
                    json[key] = value;
                }
            }
            return JSON.stringify(json);
        }
    })
</script>
{% raw %}{% endblock js %}{% endraw %}