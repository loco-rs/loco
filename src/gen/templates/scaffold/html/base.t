to: assets/views/base.html
skip_exists: true
message: "Base template was added successfully."
---

<!DOCTYPE html>
<html lang="en">

<head>
  <title>{% raw %}{% block title %}{% endblock title %}{% endraw %}</title>
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