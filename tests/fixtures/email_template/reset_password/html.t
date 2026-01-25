{% extends "base.t" %}
{% block title %}Reset Password{% endblock %}
{% block body %}
<h1>Reset Your Password</h1>
<p>Click the link below to reset your password:</p>
<p><a href="{{ resetUrl }}">Reset Password</a></p>
{% endblock %}

