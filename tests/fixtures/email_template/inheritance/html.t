{% extends "base.t" %}
{% block title %}Welcome Email{% endblock %}
{% block body %}
<h1>Hello {{ name }}!</h1>
<p>Your verification token is: <strong>{{ verifyToken }}</strong></p>
{% endblock %}

