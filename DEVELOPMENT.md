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

## Publishing a new version

**Test your changes**

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
