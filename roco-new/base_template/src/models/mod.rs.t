pub mod _entities;
{%- if settings.auth %}
pub mod users;
{%- endif %}