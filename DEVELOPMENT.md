## Blessed depdenencies maintenance and `loco doctor`

Loco contain a few major and "blessed" dependencies, these appear **both** in an app that was generated at the surface level in their `Cargo.toml` and in the core Loco framework.

If stale, may require an upgrade as a must.

Example for such dependencies:

* The `sea-orm-cli` - while Loco uses `SeaORM`, it uses the `SeaORM` CLI to generate entities, and so there may be an incompatibility if `SeaORM` has a too large breaking change between their CLI (which ships separately) and their framework. 
* `axum`
* etc.

This is why we are checking these automatically as part of `loco doctor`.

We keep minimal version requirements for these. As a maintainer, you can update these **minimal** versions, only if required in [`doctor.rs`](src/doctor.rs).



## Running Tests

Before running tests make sure that:

[ ] redis is running
[ ] starters/saas frontend package is built:

```
$ cd starters/saas/frontend
$ npm i -g pnpm
$ pnpm i && pnpm build
```

Running all tests should be done with:

```
$ cargo xtask test
```

## Rebuilding your database and local generated entities

This should write out a fresh DB structure (drops and migrates):

```
$ cargo loco db reset
```

And then, the entities generators connect to that newly minted DB, to generate a corresponding entities code:

```
$ cargo loco db entities
```

## Publishing a new version

**Test your changes**

* [ ] Ensure you have the necessary local resources, such as `DB`/`Redis`, by executing the command `cargo loco doctor  --environment test`. In case you don't have them, refer to the relevant documentation section for guidance.
* [ ] run `cargo test` on the root to test Loco itself
* [ ] cd `examples/demo` and run `cargo test` to test our "driver app" which exercises the framework in various ways
* [ ] push your changes to Github to get the CI running and testing in various additional configurations that you don't have
* [ ] CI should pass. Take note that all `starters-*` CI are using a **fixed version** of Loco and are not seeing your changes yet


**Actually bump version + test and align starters**

* [ ] in project root, run `cargo xtask bump-version` and give it the next version. Versions are without `v` prefix. Example: `0.1.3`. 
* [ ] Did the xtask testing workflow fail?
  * [ ] YES: fix errors, and re-run `cargo xtask bump-version` **with the same version as before**.
  * [ ] NO: great, move to publishing
* [ ] Your repo may be dirty with fixes. Now that tests are passing locally commit the changes. Then run `cargo publish` to publish the next Loco version (remember: the starters at this point are pointing to the **next version already**, so we don't want to push until publish finished)
* [ ] When publish finished successfully, push your changes to github
* [ ] Wait for CI to finish. You want to be focusing more at the starters CI, because they will now pull the new version.
* [ ] Did CI fail?
  * [ ] YES: This means you had a circumstance that's not predictable (e.g. some operating system issue). Fix the issue and **repeat the bumping process, advance a new version**.
  * [ ] NO: all good! you're done.

**Book keeping**

* [ ] Update changelog: (1) move vnext to be that new version of yours, (2) create a blank vnext
* [ ] Think about if any of the items in the new version needs new documentation or update to the documentation -- and do it
## Errors

Errors are done with `thiserror`. We adopt a minimalistic approach to errors.

* We try to have _one error kind_ for the entirety of Loco.
* Errors that cannot be handled, are _informative_ and so can be opaque (we don't offer deep matching on those)
* Errors that can be handled and reasoned upon should be able to be matched and extract good knowledge from
* To users, error should _not be cryptic_, and should indicate how to fix issues as much as possible, or point to the issue precisely


### Auto conversions

When possible use `from` conversions.

```rust
    #[error(transparent)]
    JSON(#[from] serde_json::Error),
```

When complicated, implement a `From` trait yourself. This is done to _centralize_ errors into one place and not litter needless `map_err` code which holds error conversion logic (an exception is Context, see below).


### Context

When you know a user might need context, resort to manually shaping the error with extra information. First, define the error:

```rust
    #[error("cannot parse `{1}`: {0}")]
    YAMLFile(#[source] serde_yaml::Error, String),
```

Then, shape it:

```rust
  serde_yaml::from_str(&rendered)
      .map_err(|err| Error::YAMLFile(err, selected_path.to_string_lossy().to_string()))
```

In this example, the information about where `rendered` came from was long lost at the `serde_yaml::from_str` callsite. Which is why errors were cryptic indicating bad YAML format, but not where it comes from (which file).

In this case, we duplicate the YAML error type, leave one of those for auto conversions with `from`, where we don't have a file, and create a new specialized error type with the file information: `YAMLFile`.

## The `CONTRIBUTORS` comment

Some files contain a special `CONTRIBUTORS` comment. This comment should
contain context, special notes for that module, and a checklist if needed, so please make sure to follow it.

