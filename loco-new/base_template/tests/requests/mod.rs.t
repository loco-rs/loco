{%- if settings.auth -%}
mod auth;
mod prepare_data;
{%- else -%}
mod home;
{%- endif -%}
{%- if settings.asset and settings.asset.kind == "server" %}
mod page;
{%- endif -%}
