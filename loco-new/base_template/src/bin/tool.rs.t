use loco_rs::cli;
use {{settings.module_name}}::app::App;
{%- if settings.db %}
use migration::Migrator;
{%- endif %}

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    {%- if settings.db %}
    cli::main::<App, Migrator>().await
    {%- else %}
    cli::main::<App>().await    
    {%- endif %}
}
