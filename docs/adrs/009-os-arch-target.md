# ADR 009: OS/architecture target

Problem: most compiled software will be compiled for a specific OS, architecture, and maybe version of libc.
If we don't handle that, rbt's eventual remote cache may end up pulling in a built binary for the wrong triple!

To handle this, we're going to assume that all software is built for the host's OS and architecture unless otherwise specified.

This gets us safety (we should never load a binary that can't be run on the current host) at the cost of having to rebuild more.
This seems fine according to our design principles: it's transparent and approachable at a slight cost to speed.

## Implementation Suggestions

One implementation might look like adding a `target` field to `Job`, which would be a type that looked like this:

```elixir
Target : [Host, Generic]
```

`Host` would be the default, and rbt would calculate it in Rust-land so that we did not have to deal with effects.

## Cross-Compilation

This proposal doesn't specify how to do cross-compilation, although that may be desirable.
In that case, we may be able to extend the `Target` field, or we may need to add something else.
That's work for another ADR!
