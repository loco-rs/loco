# Loco xtask

The Loco xtask serves as a loco development helper, streamlining various tasks on the library, such as running all tests with a single command and preparing for a new release and maybe more.

_Current Progress:_
The xtask is an under development with the following goals:

- Implement all githbu actions CI functions

  - allow to run xtask test all which run all our github action ci locally.
  - Migrate all yaml ci code to trigger xtask

    ```
    ...
       steps:
      - name: Checkout the code
        uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          profile: ${{ env.TOOLCHAIN_PROFILE }}
          toolchain: ${{ env.RUST_TOOLCHAIN }}
          override: true
          components: rustfmt
      - run: cargo run test --folder [.|/examples/demo|starters/saas|...] action fmt|clippy|test|all
        working-directory: ./xtask
    ...

    ```

- Release version: still under investigation
