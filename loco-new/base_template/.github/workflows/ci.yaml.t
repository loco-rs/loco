name: CI
on:
  push:
    branches:
      - master
      - main
  pull_request:

env:
  RUST_TOOLCHAIN: stable
  TOOLCHAIN_PROFILE: minimal

jobs:
  rustfmt:
    name: Check Style
    runs-on: ubuntu-latest

    permissions:
      contents: read

    steps:
      - name: Checkout the code
        uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: ${% raw %}{{ env.RUST_TOOLCHAIN }}{% endraw %}
          components: rustfmt
      - name: Run cargo fmt
        uses: actions-rs/cargo@v1
        with:
          command: fmt
          args: --all -- --check

  clippy:
    name: Run Clippy
    runs-on: ubuntu-latest

    permissions:
      contents: read

    steps:
      - name: Checkout the code
        uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: ${% raw %}{{ env.RUST_TOOLCHAIN }}{% endraw %}
      - name: Setup Rust cache
        uses: Swatinem/rust-cache@v2
      - name: Run cargo clippy
        uses: actions-rs/cargo@v1
        with:
          command: clippy
          args: --all-features -- -D warnings -W clippy::pedantic -W clippy::nursery -W rust-2018-idioms

  test:
    name: Run Tests
    runs-on: ubuntu-latest

    permissions:
      contents: read
    
    services:
      redis:
        image: redis
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - "6379:6379"
      postgres:
        image: postgres
        env:
          POSTGRES_DB: postgres_test
          POSTGRES_USER: postgres
          POSTGRES_PASSWORD: postgres
        ports:
          - "5432:5432"
        # Set health checks to wait until postgres has started
        options: --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - name: Checkout the code
        uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: ${% raw %}{{ env.RUST_TOOLCHAIN }}{% endraw %}
    
      {%- if settings.asset %}
      {%- if settings.asset.kind == "client" %}
      - name: Setup node
        uses: actions/setup-node@v4
        with:
          node-version: ${% raw %}{{matrix.node-version}}{% endraw %}
      - name: Build frontend
        run: npm install && npm run build
        working-directory: ./frontend
      {%- endif -%}
      {%- endif %}
      - name: Setup Rust cache
        uses: Swatinem/rust-cache@v2
      - name: Run cargo test
        uses: actions-rs/cargo@v1
        with:
          command: test
          args: --all-features --all
        env:
          REDIS_URL: redis://localhost:${% raw %}{{job.services.redis.ports[6379]}}{% endraw %}
          DATABASE_URL: postgres://postgres:postgres@localhost:5432/postgres_test
          
