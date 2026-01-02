{% set module_name = name | snake_case -%}
{% set struct_name = module_name | pascal_case -%}
to: "src/mailers/shared/base.t"
skip_exists: true
---
<!DOCTYPE html>
<html>
<head>
    <title>{% raw %}{% block title %}{% endraw %}Email{% raw %}{% endblock %}{% endraw %}</title>
    <style>
        body { font-family: Arial, sans-serif; line-height: 1.6; color: #333; }
        .container { max-width: 600px; margin: 0 auto; padding: 20px; }
        .footer { margin-top: 40px; padding-top: 20px; border-top: 1px solid #eee; color: #666; font-size: 12px; }
    </style>
</head>
<body>
    <div class="container">
        {% raw %}{% block body %}{% endraw %}{% raw %}{% endblock %}{% endraw %}
        <div class="footer">
            {% raw %}{% block footer %}{% endraw %}{% raw %}{% endblock %}{% endraw %}
        </div>
    </div>
</body>
</html>

