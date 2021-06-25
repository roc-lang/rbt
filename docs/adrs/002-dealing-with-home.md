# ADR 002: Dealing With `HOME`

Decision: we'll create a fake home directory and set it to `HOME`.
After the job finishes, we'll discard it.

If the job writes files in `HOME`, we'll issue a warning.

The directory structure of the working directory will look like this:

```
build_working_directory
├── rbt-exec
└── rbt-home
```

Needs thought still:

- Should this be a warning or an error?
- Are `exec` and `home` the right names for these things?

## Background and Motivation

When we drop the environment, we also get rid of `HOME`.
That can cause problems if, for example, your build tool relies on reading a file in `~/.config` or storing stuff in `~/.cache`, to say nothing of the tools just store stuff straight in `~`.

This can be a source of irreproducibility: you cannot know if you've completely specified your dependencies if the tool persists state between runs.
What happens if you remove an input but the result is cached?
It could work on your computer, but not CI (or your coworker's computer.)

## Things Other Build Systems Do

### Nix / NixOS

Nix sets `HOME` to `/homeless-shelter` (which is... not a great name, in my opinion.)
They don't create this directory, it just doesn't exist.
That means tools that try to read/write to it will just fail, and then you have to search for what's going on to figure out the mechanic here.

Consequences of this decision:

- They never have to worry about a fake `HOME` being polluted with files
- People have a worse experience the first time they try to build some software with a tools that reads/writes in `HOME`.
