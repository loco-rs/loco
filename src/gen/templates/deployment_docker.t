to: "dockerfile"
skip_exists: true
message: "Dockerfile generated successfully."
---
FROM rust:1.74-slim as builder

WORKDIR /usr/src/

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /usr/app

COPY --from=builder /usr/src/config /usr/app/config
COPY --from=builder /usr/src/target/release/{{pkg_name}}-cli /usr/app/{{pkg_name}}-cli

ENTRYPOINT ["/usr/app/{{pkg_name}}-cli"]