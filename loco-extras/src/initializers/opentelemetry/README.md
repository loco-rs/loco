# Opentelemetry Initializer

This initializer is responsible for initializing tracing with opentelemetry and adding it as a middleware in axum. 

Because opentelemetry tracing initializer is not compatible with loco's default tracing initialization we must turn off the default logging.

````
fn init_logger(_config: &config::Config, _env: &Environment) -> Result<bool> {
    Ok(true)
}
````