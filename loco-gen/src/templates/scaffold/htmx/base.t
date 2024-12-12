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

<body class="prose p-10">
  <div id="content">
    {% raw %}{% block content %}
    {% endblock content %}{% endraw %}
  </div>

  {% raw %}{% block js %}

  {% endblock js %}{% endraw %}
</body>

</html>