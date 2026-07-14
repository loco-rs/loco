{%- if settings.auth -%} 
pub mod auth;
{%- else -%} 
pub mod home;
{%- endif -%}