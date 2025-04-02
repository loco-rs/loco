to: assets/views/base.html
skip_exists: true
message: "Base template was added successfully."
---

<!DOCTYPE html>
<html lang="en">

<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>{% raw %}{% block title %}{% endblock title %}{% endraw %}</title>

  <script src="https://unpkg.com/htmx.org@2.0.0/dist/htmx.min.js"></script>
  <script src="https://cdn.tailwindcss.com?plugins=forms,typography,aspect-ratio,line-clamp"></script>
  {% raw %}{% block head %}{% endraw %}

  {% raw %}{% endblock head %}{% endraw %}
</head>

<body class="min-h-screen bg-background font-sans antialiased">
    <div class="relative flex min-h-screen flex-col bg-background">
        <div class="themes-wrapper bg-background">
            <main>
                <div class="flex flex-1 flex-col gap-4 p-5 pt-5">
                    <h1 class="scroll-m-20 text-3xl font-bold tracking-tight">
                        {% raw %}{% block page_title %}{% endblock page_title %}{% endraw %}
                    </h1>
                    {% raw %}{% block content %}
                    {% endblock content %}{% endraw %}
                </div>
            </main>
        </div>
    </div>
  {% raw %}{% block js %}

  {% endblock js %}{% endraw %}

  <script>
  htmx.defineExtension('submitjson', {
        onEvent: function (name, evt) {
            if (name === "htmx:configRequest") {
                evt.detail.headers['Content-Type'] = "application/json"
            }
        },
        encodeParameters: function (xhr, parameters, elt) {
                const json = {};
                for (const [key, inputValue] of Object.entries(parameters)) {
                    let origInputType = elt.querySelector(`[name=${key}]`).type;
                    const customType = elt.querySelector(`[name=${key}]`).getAttribute("custom_type");

                    let value = inputValue;
                    if (customType == "array" && !Array.isArray(inputValue)) {
                        value = [inputValue]
                    }

                    if (origInputType === 'number') {
                        if (Array.isArray(value)) {
                            json[key] = Object.values(value).map(str => parseFloat(str))
                        } else {
                            json[key] = parseFloat(value)
                        }
                    } else if (origInputType === 'checkbox') {
                        const val = elt.querySelector(`[name=${key}]`).checked;
                        json[key] = val
                    } else if (customType === 'blob') {
                        json[key] = value.split(",").map(num => parseInt(num, 10));
                    } else {
                        json[key] = value;
                    }
                }
                return JSON.stringify(json);
            }
  })
  function confirmDelete(event, delete_url, redirect_to) {
        event.preventDefault();
        if (confirm("Are you sure you want to delete this item?")) {
            var xhr = new XMLHttpRequest();
            xhr.open("DELETE", delete_url, true);
            xhr.onreadystatechange = function () {
                if (xhr.readyState == 4 && xhr.status == 200) {
                    window.location.href = redirect_to;
                }
            };
            xhr.send();
        }
    }

    document.addEventListener('DOMContentLoaded', function () {
            document.querySelectorAll('.add-more').forEach(button => {
                button.addEventListener('click', function () {
                    const group = this.getAttribute('data-group');
                    const first = document.getElementById(`${group}-inputs`).querySelector('input');
                    if (first) {
                        const clonedInput = first.cloneNode();
                        clonedInput.value = '';
                        const container = document.getElementById(`${group}-inputs`);
                        container.appendChild(clonedInput);
                    } 
                });
            });
    });

    document.body.addEventListener('htmx:responseError', function (event) {
        const target = document.querySelector('#error-message');
        const errorResponse = event.detail.xhr.response;
        target.innerHTML = errorResponse
    });
  </script>
</body>

</html>