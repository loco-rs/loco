# Loco configuration file documentation

# Application logging configuration
logger:
  # Enable or disable logging.
  enable: true
  # Enable pretty backtrace (sets RUST_BACKTRACE=1)
  pretty_backtrace: true
  # Log level, options: trace, debug, info, warn or error.
  level: {{ get_env(name="LOG_LEVEL", default="debug") }}
  # Define the logging format. options: compact, pretty or json
  format: compact
  # By default the logger has filtering only logs that came from your code or logs that came from `loco` framework. to see all third party libraries
  # Uncomment the line below to override to see all third party libraries you can enable this config and override the logger filters.
  # override_filter: trace

# Web server configuration
server:
  # Port on which the server will listen. the server binding is 0.0.0.0:{PORT}
  port: {{ get_env(name="PORT", default="5150") }}
  # Binding for the server (which interface to bind to)
  binding: {{ get_env(name="BINDING", default="localhost") }}
  # The UI hostname or IP address that mailers will point to.
  host: http://localhost
  # Out of the box middleware configuration. to disable middleware you can changed the `enable` field to `false` of comment the middleware block
  middlewares:
    request_context:
      enable: true
      session_config:
        name: "__loco_session"
        http_only: true
        same_site:
          type: Lax
        expiry: 3600
        secure: false
        path: /
      # domain: ""
      session_store:
        type: Cookie
        value: 
          private_key: {{ settings.session_private_key }}
  {%- if settings.asset %}
    {%- if settings.asset.kind == "server" %}
    static:
      enable: true
      must_exist: true
      precompressed: false
      folder:
        uri: "/static"
        path: "assets/static"
      fallback: "assets/static/404.html"
  {%- elif settings.asset.kind == "client" %}
    fallback:
      enable: false
    static:
      enable: true
      must_exist: true
      precompressed: false
      folder:
        uri: "/"
        path: "frontend/dist"
      fallback: "frontend/dist/index.html"
  {%- endif -%}

  {%- endif -%}

{%- if settings.background%}

# Worker Configuration
workers:
  # specifies the worker mode. Options:
  #   - BackgroundQueue - Workers operate asynchronously in the background, processing queued.
  #   - ForegroundBlocking - Workers operate in the foreground and block until tasks are completed.
  #   - BackgroundAsync - Workers operate asynchronously in the background, processing tasks with async capabilities.
  mode: {{settings.background.kind}}

  {% if settings.background.kind == "BackgroundQueue"%}
# Queue Configuration
queue:
  kind: Redis
  # Redis connection URI
  uri: {% raw %}{{{% endraw %} get_env(name="REDIS_URL", default="redis://127.0.0.1") {% raw %}}}{% endraw %}
  # Dangerously flush all data in Redis on startup. dangerous operation, make sure that you using this flag only on dev environments or test mode
  dangerously_flush: false
  {%- endif %}
{%- endif -%}

{%- if settings.mailer %}

# Mailer Configuration.
mailer:
  # SMTP mailer configuration.
  smtp:
    # Enable/Disable smtp mailer.
    enable: true
    # SMTP server host. e.x localhost, smtp.gmail.com
    host: {{ get_env(name="MAILER_HOST", default="localhost") }}
    # SMTP server port
    port: 1025
    # Use secure connection (SSL/TLS).
    secure: false
    # auth:
    #   user:
    #   password:
    # Override the SMTP hello name (default is the machine's hostname)
    # hello_name:
{%- endif %}

# Initializers Configuration
# initializers:
#  oauth2:
#    authorization_code: # Authorization code grant type
#      - client_identifier: google # Identifier for the OAuth2 provider. Replace 'google' with your provider's name if different, must be unique within the oauth2 config.
#        ... other fields

{%- if settings.db %}

# Database Configuration
database:
  # Database connection URI
  uri: {% raw %}{{{% endraw %} get_env(name="DATABASE_URL", default="{{settings.db.endpoint | replace(from='NAME', to=settings.package_name) | replace(from='ENV', to='development')}}") {% raw %}}}{% endraw %}
  # When enabled, the sql query will be logged.
  enable_logging: {{ get_env(name="DB_LOGGING", default="false") }}
  # Set the timeout duration when acquiring a connection.
  connect_timeout: {% raw %}{{{% endraw %} get_env(name="DB_CONNECT_TIMEOUT", default="500") {% raw %}}}{% endraw %}
  # Set the idle duration before closing a connection.
  idle_timeout: {% raw %}{{{% endraw %} get_env(name="DB_IDLE_TIMEOUT", default="500") {% raw %}}}{% endraw %}
  # Minimum number of connections for a pool.
  min_connections: {% raw %}{{{% endraw %} get_env(name="DB_MIN_CONNECTIONS", default="1") {% raw %}}}{% endraw %}
  # Maximum number of connections for a pool.
  max_connections: {% raw %}{{{% endraw %} get_env(name="DB_MAX_CONNECTIONS", default="1") {% raw %}}}{% endraw %}
  # Run migration up when application loaded
  auto_migrate: true
  # Truncate database when application loaded. This is a dangerous operation, make sure that you using this flag only on dev environments or test mode
  dangerously_truncate: false
  # Recreating schema when application loaded.  This is a dangerous operation, make sure that you using this flag only on dev environments or test mode
  dangerously_recreate: false
{%- endif %}

{%- if settings.auth %}

# Authentication Configuration
auth:
  # JWT authentication
  jwt:
    # Secret key for token generation and verification
    secret: {{20 | random_string }}
    # Token expiration time in seconds
    expiration: 604800 # 7 days
{%- endif %}
