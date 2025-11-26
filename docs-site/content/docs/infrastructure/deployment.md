+++
title = "Deployment"
date = 2021-05-01T08:00:00+00:00
updated = 2021-05-01T08:00:00+00:00
draft = false
weight = 3
sort_by = "weight"
template = "docs/page.html"

[extra]
toc = true
top = false
flair =[]
+++

Deployment is super simple in Loco, and this is why this guide is super short. Although **most of the time in development you are using `cargo`** when deploying, you use the **binary that was compiled**, there is no need for `cargo` or Rust on the target server.

## How to Deploy
First, check your Cargo.toml to see your application name:
```toml
[package]
name = "myapp" # This is your binary name
version = "0.1.0"
```

build your production binary for your relevant server architecture:

<!-- <snip id="build-command" inject_from="yaml" template="sh"> -->
```sh
cargo build --release
```
<!-- </snip>-->

And copy your binary along with your `config/` folder to the server. You can then run `myapp start` on your server.

```sh
# The binary is located in ./target/release/ after building
./target/release/myapp start
```

That's it!

We took special care that **all of your work** is embbedded in a **single** binary, so you need nothing on the server other than that.

## Review your production config

There are a few configuration sections that are important to review and set accordingly when deploying to production:

- Logger:

<!-- <snip id="configuration-logger" inject_from="code" template="yaml"> -->
```yaml
# Application logging configuration
logger:
  # Enable or disable logging.
  enable: true
  # Enable pretty backtrace (sets RUST_BACKTRACE=1)
  pretty_backtrace: true
  # Log level, options: trace, debug, info, warn or error.
  level: debug
  # Define the logging format. options: compact, pretty or json
  format: compact
  # By default the logger has filtering only logs that came from your code or logs that came from `loco` framework. to see all third party libraries
  # Uncomment the line below to override to see all third party libraries you can enable this config and override the logger filters.
  # override_filter: trace
```
<!-- </snip>-->
 

- Server:
<!-- <snip id="configuration-server" inject_from="code" template="yaml"> -->
```yaml
server:
  # Port on which the server will listen. the server binding is 0.0.0.0:{PORT}
  port: {{ get_env(name="NODE_PORT", default=5150) }}
  # The UI hostname or IP address that mailers will point to.
  host: http://localhost
```
<!-- </snip>-->


- Database:
<!-- <snip id="configuration-database" inject_from="code" template="yaml"> -->
```yaml
database:
  # Database connection URI
  uri: {{get_env(name="DATABASE_URL", default="postgres://loco:loco@localhost:5432/loco_app")}}
  # When enabled, the sql query will be logged.
  enable_logging: false
  # Set the timeout duration when acquiring a connection.
  connect_timeout: 500
  # Set the idle duration before closing a connection.
  idle_timeout: 500
  # Minimum number of connections for a pool.
  min_connections: 1
  # Maximum number of connections for a pool.
  max_connections: 1
  # Run migration up when application loaded
  auto_migrate: true
  # Truncate database when application loaded. This is a dangerous operation, make sure that you using this flag only on dev environments or test mode
  dangerously_truncate: false
  # Recreating schema when application loaded.  This is a dangerous operation, make sure that you using this flag only on dev environments or test mode
  dangerously_recreate: false
```
<!-- </snip>-->


- Mailer:
<!-- <snip id="configuration-mailer" inject_from="code" template="yaml"> -->
```yaml
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
```
<!-- </snip>-->

- Queue:
<!-- <snip id="configuration-queue" inject_from="code" template="yaml"> -->
```yaml
queue:
  kind: Redis
  # Redis connection URI
  uri: {{ get_env(name="REDIS_URL", default="redis://127.0.0.1") }}
  # Dangerously flush all data in Redis on startup. dangerous operation, make sure that you using this flag only on dev environments or test mode
  dangerously_flush: false
```
<!-- </snip>-->

- JWT secret:
<!-- <snip id="configuration-auth" inject_from="code" template="yaml"> -->
```yaml
auth:
  # JWT authentication
  jwt:
    # Secret key for token generation and verification
    secret: PqRwLF2rhHe8J22oBeHy
    # Token expiration time in seconds
    expiration: 604800 # 7 days
```
<!-- </snip>-->

## Running `loco doctor`

You can run `loco doctor` in your server to check the connection health of your environment. 

```sh
$ myapp doctor --production
```

## Generate

Loco offers a deployment template enabling the creation of a deployment infrastructure.

```sh
$ cargo loco generate deployment --help
Generate a deployment infrastructure

Usage: myapp-cli generate deployment [OPTIONS] <KIND>

Arguments:
  <KIND>  [possible values: docker, shuttle, nginx]
```

<!-- <snip id="generate-deployment-command" inject_from="yaml" template="sh"> -->

```sh
cargo loco generate deployment docker

added: "Dockerfile"
added: ".dockerignore"
* Dockerfile generated successfully.
* Dockerignore generated successfully
```

<!-- </snip>-->

Deployment Options:

1. Docker:

- Generates a Dockerfile ready for building and deploying.
- Creates a .dockerignore file.

2. Shuttle:

- Generates a shuttle main function.
- Adds `shuttle-runtime` and `shuttle-axum` as dependencies.
- Adds a bin entrypoint for the deployment.

3. Nginx:

- Generates a nginx configuration file for reverse proxying.

Choose the option that best fits your deployment needs. Happy deploying!

If you have a preference for deploying on a different cloud, feel free to open a pull request. Your contributions are more than welcome!

## AWS Lambda

Loco provides a built-in AWS Lambda deployment experience similar to [Zappa](https://github.com/zappa/Zappa) for Python. Deploy your Loco application to AWS Lambda with a single command—no permanent scripts or configuration files generated.

### Prerequisites

1. **Install Cargo Lambda**:
```sh
# Using Cargo (requires Zig or Docker)
cargo install cargo-lambda
```

2. **Configure AWS credentials** using one of these methods:
   - AWS CLI: `aws configure`
   - Environment variables: `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`
   - IAM role (when running on AWS infrastructure)

### Configuration

Add the `lambda` section to your `config/development.yaml` (or any environment config):

```yaml
lambda:
  # Lambda function name (defaults to package name)
  project_name: my-loco-app
  # Memory in MB (128-10240). More memory = faster cold starts
  memory_size: 256
  # Timeout in seconds (max 900)
  timeout: 30
  # AWS region
  region: us-east-1
  # Loco environment to load in Lambda (development, production, test)
  loco_env: development
  # AWS CLI profile (optional)
  # profile_name: default
  # CPU architecture: x86_64 or arm64 (arm64 recommended for better price/performance)
  architecture: arm64
  # Enable direct HTTP access via Lambda Function URL
  function_url: true
  # Environment variables passed to Lambda
  environment:
    RUST_LOG: info
    DATABASE_URL: "postgres://user:pass@host:5432/db"
  # IAM role ARN (optional - auto-created if not specified)
  # role_arn: arn:aws:iam::ACCOUNT_ID:role/your-role
```

### Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `project_name` | Package name | Lambda function name |
| `memory_size` | `256` | Memory allocation in MB (128-10240) |
| `timeout` | `30` | Function timeout in seconds (max 900) |
| `region` | `us-east-1` | AWS region for deployment |
| `loco_env` | `development` | Which Loco config environment to load |
| `profile_name` | - | AWS CLI profile name |
| `architecture` | `arm64` | CPU architecture (`x86_64` or `arm64`) |
| `function_url` | `true` | Enable Lambda Function URL for HTTP access |
| `environment` | `{}` | Environment variables for the Lambda function |
| `role_arn` | - | Custom IAM role ARN (auto-created if not set) |

### Deploy

Deploy your application to AWS Lambda:

```sh
cargo loco lambda deploy
```

This command:
1. Creates a temporary Lambda handler binary
2. Builds for the target architecture using `cargo lambda build`
3. Deploys to AWS Lambda with your configuration
4. Sets up a Function URL (if enabled)
5. Cleans up all temporary files

For a dry run (build only, no deployment):

```sh
cargo loco lambda deploy --dry-run
```

### Invoke

Test your deployed Lambda function:

```sh
# Default health check
cargo loco lambda invoke

# Custom payload
cargo loco lambda invoke --payload '{"httpMethod": "GET", "path": "/api/users"}'

# POST request example
cargo loco lambda invoke --payload '{"httpMethod": "POST", "path": "/api/auth/login", "body": "{\"email\":\"user@example.com\",\"password\":\"secret\"}"}'
```

### Logs

View CloudWatch logs for your Lambda function:

```sh
# Recent logs
cargo loco lambda logs

# Follow logs in real-time
cargo loco lambda logs --follow
```

### How It Works

The Lambda deployment creates a handler that:
- Wraps your Loco application with the AWS Lambda runtime
- Uses `lambda_http` for API Gateway and Lambda Function URL compatibility
- Loads configuration from the environment specified by `loco_env`
- Includes your `config/` directory in the deployment package
