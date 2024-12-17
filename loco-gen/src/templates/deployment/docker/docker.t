to: "dockerfile"
skip_exists: true
message: "Dockerfile generated successfully."
---
FROM rust:1.83-slim as builder

WORKDIR /usr/src/

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /usr/app

{% if copy_asset_folder -%}
COPY --from=builder /usr/src/{{copy_asset_folder}} /usr/app/{{copy_asset_folder}}
{% endif -%}
{% if fallback_file -%}
COPY --from=builder /usr/src/{{fallback_file}} /usr/app/{{fallback_file}}
{% endif -%}
COPY --from=builder /usr/src/config /usr/app/config
COPY --from=builder /usr/src/target/release/{{pkg_name}}-cli /usr/app/{{pkg_name}}-cli

ENTRYPOINT ["/usr/app/{{pkg_name}}-cli"]
