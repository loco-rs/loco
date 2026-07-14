{%- if settings.auth -%} 
mod auth;
mod prepare_data;
{%- else -%} 
mod home;
{%- endif -%}