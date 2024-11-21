[alias]
loco = "run --"
{%- if settings.os == "windows" %}
loco-tool = "run --bin tool --"
{% else %}
loco-tool = "run --"
{%- endif %}

playground = "run --example playground"
