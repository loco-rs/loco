# Loco xtask

The Loco xtask serves as a loco development helper, streamlining various tasks on the library, such as running all tests with a single command and preparing for a new release and maybe more.

## Bump version

To release a new Loco version, execute the following command:

```rust
cargo run bump-version VERSION
```

The `bump-version` command performs the following steps:

- Updates the Loco library in [cargo.toml](../Cargo.toml)
- Replaces all starters with ../../loco-rs to enable CI testing for the targeted release version
  - If the CI process fails, the operation is halted
- Locks all starters to the specified Loco version

### Release Steps

- Create new branch `git checkout -b bump-version-[VERSION]`
- run the following script for update all relevant resources
  ```sh
  cd xtask
  cargo run bump-version VERSION
  ```
- push the branch and wait for CI will pass
- publish the new crate
- merge to to main
