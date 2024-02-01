# Starters Overview

Loco starters provide a straightforward way to initiate a Loco project, minimizing user effort and facilitating the seamless adoption of Loco.

## How It Works

The [loco-cli](../loco-cli/README.md) dynamically searches for all available starter projects in the starters root path during runtime. It then prompts the user to choose from the available starters.

Starters **must work**, as users often start with these projects, and it's crucial to provide a positive first impression :).

When releasing a new version of the Loco library, we utilize xtask utilities, which automatically update the Loco version and the starters, locking them to the updated version.

## When to Update a Starter

Ideally, **avoid** making changes to the starters. Starters should have minimal implementations.

## When to Add a New Starter

We recommend not submitting a pull request (PR) with a new starter right away. Instead, feel free to open a [PR](https://github.com/loco-rs/loco/issues/new/choose) and as and consult with us before investing your time.

## Handling Breaking Changes

When making breaking changes, there is generally no need to update the starters. During version bumps, we thoroughly test the starter projects and make any necessary adjustments if needed.
