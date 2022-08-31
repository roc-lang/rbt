# 🐸 rbt

A tool for building and testing projects without redoing work unnecessarily.

A very early work in progress!

> RBT is short for Roc Build Tool, and can be pronounced "ribbit." 🐸

## Hacking

For now, you'll need to have [the `nix` package manager](https://nixos.org/download.html) installed.
After that, you can either `nix-shell` to get into a development shell.

If you want this to happen automatically, one way is to:

1. [install `direnv`](https://direnv.net/)
2. `echo "use nix" >> .envrc`
3. run `direnv allow`

At which point, you'll enter a shell whenever you `cd` here.

### Repo Maintenance

#### Updating dev dependencies

rbt versions `roc` and `nixpkgs` using [`niv`](https://github.com/nmattia/niv).

To update these deps, get into a dev shell and run `niv update`, or `niv update roc` or `niv update nixpkgs` to just update one or the other.

### Updating `roc_std`

To simplify dependency management, rbt vendors a copy of Roc's standard library at `vendor/roc_std`.
We keep this in sync with the Roc version from niv, but check in the files to make life easier in CI.

To update this code, get into a dev shell and run `sync-roc-std`, then commit the changes.

### Updating `src/glue.rs`

Regenerate bindings between Roc (whose entrypoint is `Package-Config.roc`) and `src/glue.rs` by running `sync-glue` from a dev shell.

If everything compiles and works, then fix any Clippy errors that have shown up in the generated code, probably by putting a `#![allow(clippy)]` directive at the top of the file.
