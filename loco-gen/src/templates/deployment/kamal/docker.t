to: "Dockerfile"
skip_exists: true
message: "Dockerfile generated successfully."

injections:
- into: config/development.yaml
  after: "  port: 5150"
  content: "  # Expose Server on all interfaces\n  binding: 0.0.0.0"
  
---

FROM rust:1.84-slim as builder

WORKDIR /usr/src/

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim
# Install required system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libpq-dev \
    libssl-dev \
    curl \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install sea-orm-cli
RUN cargo install sea-orm-cli
WORKDIR /usr/app
{% if copy_asset_folder -%}
COPY --from=builder /usr/src/{{copy_asset_folder}} /usr/app/{{copy_asset_folder}}
{% endif -%}
COPY --from=builder /usr/src/assets/views /usr/app/assets/views
{% if fallback_file -%}
COPY --from=builder /usr/src/{{fallback_file}} /usr/app/{{fallback_file}}
{% endif -%}
COPY --from=builder /usr/src/config /usr/app/config
COPY --from=builder /usr/src/target/release/{{pkg_name}}-cli /usr/app/{{pkg_name}}-cli

ENTRYPOINT ["/usr/app/{{pkg_name}}-cli","start"]