pub mod app;
pub mod controllers;
pub mod initializers;
{%- if settings.mailer %}
pub mod mailers;
{%- endif %}
{%- if settings.db %}
pub mod models;
{%- endif %}
pub mod tasks;
pub mod views;
{%- if settings.background %}
pub mod workers;
{%- endif %}