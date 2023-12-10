to: "dockerfile"
skip_exists: true
message: "Dockerfile generated successfully."
---
FROM rust:1.74-slim

WORKDIR /usr/src/

COPY . .

RUN cargo build --release

ENTRYPOINT ["./target/release/{{pkg_name}}"]