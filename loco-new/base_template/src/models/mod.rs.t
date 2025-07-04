pub mod _entities;
{%- if settings.auth %}
pub mod users;
pub mod cli_user_create;
{%- endif %}