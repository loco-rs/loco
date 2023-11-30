+++
title = "Hello World"
description = "Introducing Doks, a Hugo theme helping you build modern documentation websites that are secure, fast, and SEO-ready — by default."
date = 2021-05-01T09:19:42+00:00
updated = 2021-05-01T09:19:42+00:00
draft = false
template = "blog/page.html"

[taxonomies]
authors = ["Rustaceans"]

[extra]
lead = "This is the source code of the traditional <b>Hello World</b> program."
+++

```rust
// This is a comment, and is ignored by the compiler

// This is the main function
fn main() {
    // Statements here are executed when the compiled binary is called

    // Print text to the console
    println!("Hello World!");
}
```

`println!` is a macro that prints text to the console.

A binary can be generated using the Rust compiler: `rustc`.

```bash
$ rustc hello.rs
```

`rustc` will produce a `hello` binary that can be executed.

```bash
$ ./hello
Hello World!
```
