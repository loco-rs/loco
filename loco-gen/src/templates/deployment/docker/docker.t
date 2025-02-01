to: "dockerfile"
skip_exists: true
message: "Dockerfile generated successfully."

injections:
- into: config/development.yaml
  remove_lines: |
    # Binding for the server (which interface to bind to)
    binding: {{ get_env(name="BINDING", default="localhost") }}
  content: |
    |  # Binding for the server (which interface to bind to)
    |  binding: {{ get_env(name="BINDING", default="0.0.0.0") }}

---

FROM rust:1.84-slim as builder

WORKDIR /usr/src/

COPY . .

{% if is_client_side_rendering -%}
RUN apt-get update && apt-get install -y curl ca-certificates

# Install Node.js using the latest available version from NodeSource.
# In production, replace "setup_current.x" with a specific version
# to avoid unexpected breaking changes in future releases.
RUN curl -fsSL https://deb.nodesource.com/setup_current.x | bash - && \
    apt-get install -y nodejs
RUN cd frontend && npm install && npm run build
{% endif -%}

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /usr/app

{% for path in copy_paths -%}
COPY --from=builder /usr/src/{{path}} {{path}}
{% endfor -%}
COPY --from=builder /usr/src/config config
COPY --from=builder /usr/src/target/release/{{pkg_name}}-cli {{pkg_name}}-cli

ENTRYPOINT ["/usr/app/{{pkg_name}}-cli","start"]