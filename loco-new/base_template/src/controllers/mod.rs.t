{%- if settings.auth -%}
pub mod auth;
{%- else -%}
pub mod home;
{%- endif -%}
{%- if settings.asset and settings.asset.kind == "server" %}
pub mod page;
{%- endif -%}
