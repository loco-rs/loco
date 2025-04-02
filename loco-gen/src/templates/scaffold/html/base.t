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
  <script src="https://cdn.tailwindcss.com?plugins=forms,typography,aspect-ratio,line-clamp"></script>
  {% raw %}{% block head %}{% endraw %}

  {% raw %}{% endblock head %}{% endraw %}
</head>

<body class="min-h-screen bg-background font-sans antialiased">
    <div class="relative flex min-h-screen flex-col bg-background">
        <div class="themes-wrapper bg-background">
                <main class="relative flex min-h-svh flex-1 flex-col bg-background peer-data-[variant=inset]:min-h-[calc(100svh-theme(spacing.4))] md:peer-data-[variant=inset]:m-2 md:peer-data-[state=collapsed]:peer-data-[variant=inset]:ml-2 md:peer-data-[variant=inset]:ml-0 md:peer-data-[variant=inset]:rounded-xl md:peer-data-[variant=inset]:shadow">
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
  </script>
</body>

</html>