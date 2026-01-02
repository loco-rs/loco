<!DOCTYPE html>
<html>
<head>
    <title>{% block title %}Email{% endblock %}</title>
    <style>
        body { font-family: Arial, sans-serif; }
        .footer { color: #666; font-size: 12px; }
    </style>
</head>
<body>
    {% block body %}{% endblock %}
    <div class="footer">
        {% block footer %}© 2024 My Company{% endblock %}
    </div>
</body>
</html>

