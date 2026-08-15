# Loco configuration file documentation
#
# This is the production environment. It differs from development in one
# deliberate way: anything that is a secret or an address takes no default.
# `get_env(name="X")` with no `default` fails at startup if `X` is unset, which
# is what you want — a production app that silently falls back to a development
# secret or a localhost database is worse than one that refuses to boot.
#
# The variables this file requires:
#   DATABASE_URL, JWT_SECRET, HOST, and the MAILER_* / REDIS_URL set below,
#   depending on which features you generated.

# Application logging configuration
logger:
  # Enable or disable logging.
  enable: true
  # Enable pretty backtrace (sets RUST_BACKTRACE=1). Off in production: it
  # costs performance and puts source paths into your logs.
  pretty_backtrace: false
  # Log level, options: trace, debug, info, warn or error.
  level: <%= get_env(name="LOG_LEVEL", default="info") %>
  # Define the logging format. options: compact, pretty or json
  # `json` is the machine-readable one, for a log aggregator.
  format: json
  # By default the logger has filtering only logs that came from your code or logs that came from `loco` framework. to see all third party libraries
  # Uncomment the line below to override to see all third party libraries you can enable this config and override the logger filters.
  # override_filter: trace

# Web server configuration
server:
  # Port on which the server will listen. the server binding is 0.0.0.0:{PORT}
  port: <%= get_env(name="PORT", default="5150") %>
  # Binding for the server (which interface to bind to).
  # `0.0.0.0`, not `localhost`: inside a container or a VM, a server bound to
  # loopback is unreachable from outside it.
  binding: <%= get_env(name="BINDING", default="0.0.0.0") %>
  # The UI hostname or IP address that mailers will point to.
  # Required: links in outgoing mail have to name the real host.
  host: <%= get_env(name="HOST") %>
  # Out of the box middleware configuration. to disable middleware you can changed the `enable` field to `false` of comment the middleware block
  middlewares:
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
  mode: {{settings.background.mode}}

  {% if settings.background.mode == "BackgroundQueue" %}
# Queue Configuration
queue:
  kind: {{settings.background.queue_kind}}
  {% if settings.background.queue_kind == "Redis" %}
  # Redis connection URI
  uri: <%= get_env(name="REDIS_URL") %>
  # Dangerously flush all data in Redis on startup. dangerous operation, make sure that you using this flag only on dev environments or test mode
  dangerously_flush: false
  {% elif settings.background.queue_kind == "Postgres" %}
  # Postgres connection URI. Commonly the same value as DATABASE_URL.
  uri: <%= get_env(name="QUEUE_URL") %>
  # Dangerously flush all data in the queue table on startup. dangerous operation, make sure that you using this flag only on dev environments or test mode
  dangerously_flush: false
  {% elif settings.background.queue_kind == "Sqlite" %}
  # SQLite connection URI
  uri: <%= get_env(name="QUEUE_URL") %>
  # Dangerously flush all data in the queue table on startup. dangerous operation, make sure that you using this flag only on dev environments or test mode
  dangerously_flush: false
  {% endif %}
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
    host: <%= get_env(name="MAILER_HOST") %>
    # SMTP server port
    port: <%= get_env(name="MAILER_PORT", default="587") %>
    # Use secure connection (SSL/TLS).
    secure: true
    auth:
      user: <%= get_env(name="MAILER_USER") %>
      password: <%= get_env(name="MAILER_PASSWORD") %>
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
  # Database connection URI. Required — there is no safe default for a
  # production database.
  uri: <%= get_env(name="DATABASE_URL") %>
  # When enabled, the sql query will be logged.
  enable_logging: <%= get_env(name="DB_LOGGING", default="false") %>
  # Set the timeout duration when acquiring a connection.
  connect_timeout: <%= get_env(name="DB_CONNECT_TIMEOUT", default="500") %>
  # Set the idle duration before closing a connection.
  idle_timeout: <%= get_env(name="DB_IDLE_TIMEOUT", default="500") %>
  # Minimum number of connections for a pool.
  min_connections: <%= get_env(name="DB_MIN_CONNECTIONS", default="1") %>
  # Maximum number of connections for a pool. Serving real traffic on the
  # development default of 1 serializes every request behind one connection.
  max_connections: <%= get_env(name="DB_MAX_CONNECTIONS", default="10") %>
  # Run migration up when application loaded.
  # Convenient for single-instance deploys. Turn it off if you run more than
  # one instance or migrate as a separate release step, so instances do not
  # race to migrate the same database.
  auto_migrate: <%= get_env(name="DB_AUTO_MIGRATE", default="true") %>
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
    # Secret key for token generation and verification. Required: a secret
    # committed to the repository is a secret your users can forge tokens with.
    secret: <%= get_env(name="JWT_SECRET") %>
    # Token expiration time in seconds
    expiration: 604800 # 7 days
{%- endif %}
