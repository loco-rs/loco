{% set file_name = name |  snake_case -%}
{% set module_name = file_name | pascal_case -%}
to: assets/views/{{file_name}}/list.html
skip_exists: true
message: "{{file_name}} list view was added successfully."
---
{% raw %}{% extends "base.html" %}{% endraw %}

{% raw %}{% block title %}{% endraw %}
List of {{file_name}}
{% raw %}{% endblock title %}{% endraw %}

{% raw %}{% block page_title %}{% endraw %}
{{file_name}}
{% raw %}{% endblock page_title %}{% endraw %}

{% raw %}{% block content %}{% endraw %}
<div class="mb-10">

    {% raw %}{% if items %}{% endraw %}

    <div class="mb-5">
        <div class="relative w-full overflow-auto">
            <table class="w-full caption-bottom text-sm">
                <thead class="[&amp;_tr]:border-b">
                    <tr class="border-b transition-colors hover:bg-muted/50">
                        {% for column in columns -%}
                        <th class="h-10 px-2 text-left align-middle font-medium text-muted-foreground [&amp;:has([role=checkbox])]:pr-0 [&amp;>[role=checkbox]]:translate-y-[2px] w-[100px]">
                            {% raw %}{{"{% endraw %}{{column.0}}{% raw %}" | capitalize }}{% endraw %}
                        </th>
                        {% endfor -%}
                        <th class="h-10 px-2 text-left align-middle font-medium text-muted-foreground [&amp;:has([role=checkbox])]:pr-0 [&amp;>[role=checkbox]]:translate-y-[2px] w-[100px]">
                           Actions
                        </th>
                    </tr>
                </thead>
                <tbody class="[&amp;_tr:last-child]:border-0">
                   {% raw %}{% for item in items %}{% endraw %}
                    <tr class="border-b transition-colors hover:bg-muted/50">
                        {% for column in columns -%}
                          <td
                            class="p-2 align-middle  font-medium">
                            {% raw %}{{item.{% endraw %}{{column.0}}{% raw %}}}{% endraw %}
                        </td>
                        {% endfor -%}
                        <td>
                            <a href="/{{name | plural}}/{% raw %}{{ item.id }}{% endraw %}/edit">Edit</a>
                        </td>
                    </tr>
                    {% raw %}{% endfor %}{% endraw %}
                </tbody>
            </table>
        </div>
    
        <div class="flex">
            <div class="ml-auto  p-4">
                <a href="/{{name | plural}}/new"
                    class="mt-5 bg-blue-500 text-white bg-primary-600 hover:bg-primary-700 focus:ring-4 focus:outline-none focus:ring-primary-300 font-medium rounded-lg text-sm px-5 py-2.5 text-center dark:bg-primary-600 dark:hover:bg-primary-700 dark:focus:ring-primary-800">
                    Create
                </a>
            </div>
        </div>
    </div>

    {% raw %}{% else %}{% endraw %}

    <div class="mt-10 flex items-center justify-center">
        <div class="bg-white rounded-lg shadow-lg p-8 max-w-4xl w-full flex flex-col items-center">
            <h3 class="font-bold text-lg">Nothing Here Yet</h3>
            There are no records to display. Add a new record to get started!
            <a href="/{{name | plural}}/new"
            class="mt-5 bg-blue-500 text-white bg-primary-600 hover:bg-primary-700 focus:ring-4 focus:outline-none focus:ring-primary-300 font-medium rounded-lg text-sm px-5 py-2.5 text-center dark:bg-primary-600 dark:hover:bg-primary-700 dark:focus:ring-primary-800">
            Create
        </a>
        </div>
    </div>
   
    {% raw %}{% endif %}{% endraw %}

    
</div>
{% raw %}{% endblock content %}{% endraw %}
