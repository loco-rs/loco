+++
title = "Loco development on NixOS"
description = "Set up your development shell with this nix flake to hit the ground running with Loco"
date = 2024-10-25T10:00:00+01:00
updated = 2024-10-25T10:00:00+01:00
draft = false
template = "blog/page.html"

[taxonomies]
authors = ["charludo"]

+++

## Overview

1. Create new Loco project
2. Add the `flake.nix`
3. Enter the development environment
4. Start Loco

## Create new Loco project

1. Run `loco new` to create a new project
2. Navigate through the instructions.

Note that the flake currently supports neither `postgres` nor `redis`, so make your choices accordingly.
Of course, if you have `redis`/`postgres` already running on your system, both are supported as usual - this flake just does not set them up for you!

## Add the `flake.nix`

Add the following as the contents of a new file `flake.nix`:

```nix
{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { fenix, nixpkgs, utils, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          system = system;
        };
        toolchain = fenix.packages.${system}.latest;

        # sea-orm-cli is on nixpkgs, but in a version too old for use with loco.rs
        sea-orm-cli = pkgs.rustPlatform.buildRustPackage rec {
          pname = "sea-orm-cli";
          version = "1.1.0";

          buildInputs = with pkgs; [ openssl ];
          nativeBuildInputs = with pkgs; [ pkg-config ];

          src = pkgs.fetchCrate {
            inherit pname version;
            hash = "sha256-qwWXHWo3gist1pTN5GlvjwyzXDLoKYcEEspy2gxJheA=";
          };
          cargoHash = "sha256-zKzJp1dBJnIWXxmx1JTiiolOydDVhJGM68zBZ3/BqAI=";
        };
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            loco-cli

            pnpm
            nodejs

            (toolchain.withComponents [
              "cargo"
              "clippy"
              "rust-src"
              "rustc"
              "rustfmt"
              "rust-analyzer"
            ])
          ] ++ [ sea-orm-cli ];
        };
      });
}
```

This flake will provide you with everything you need to develop a Loco app with any of the supported frontends.

## Enter the development environment

1. Run `nix develop` inside the project directory.
2. Wait for a couple of seconds (building everything is only required the first time you enter the development shell).
3. If you don't want to have to invoke `nix develop` each time you want to work on your project, you could use a tool like `direnv` with the following `.envrc` file:
    ```bash
    ${DIRENV_DISABLE:+exit}
    export DIRENV_DISABLE="1"
    if [ -f flake.nix ]; then
      use flake .
    fi
    ```

## Start Loco

1. Start Loco with `cargo loco start`
2. Open http://localhost:5150/

Congrats, you are up and running with Loco on NixOS! :tada:
