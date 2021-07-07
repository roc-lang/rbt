# ADR 004: Symlinking

Decision: we're going to set up files in `rbt-workspace` (see [ADR #2](./002-dealing-with-home.md)) using symlinks instead of copying things over.
This seems to strike a good balance between performance and safety.

We may revisit this eventually (e.g. for a container-based executor.)

## Background and Motivation

In ADR #1, we decided that we were going to isolate the filesystem.
Our first approach was to copy files.
However, there's a trade-off we can take: symlinking files will be much faster, but open us up to issues as symlinks cannot be made read-only.
That means that a tool could modify the source files during a build!

But let's see what we could get by symlinking instead of copying.
For large files (100 files of 1MB each):

```
$ hyperfine ./copy-all.sh ./symlink-all.sh
Benchmark #1: ./copy-all.sh
  Time (mean ± σ):     364.9 ms ±  69.2 ms    [User: 16.2 ms, System: 197.7 ms]
  Range (min … max):   211.9 ms … 441.7 ms    12 runs

Benchmark #2: ./symlink-all.sh
  Time (mean ± σ):      52.4 ms ±   7.9 ms    [User: 11.3 ms, System: 33.1 ms]
  Range (min … max):    38.1 ms …  80.5 ms    59 runs

Summary
  './symlink-all.sh' ran
    6.97 ± 1.68 times faster than './copy-all.sh'
```

For smaller files (100 files of 16KB each):

```
$ hyperfine ./copy-all.sh ./symlink-all.sh
Benchmark #1: ./copy-all.sh
  Time (mean ± σ):      42.4 ms ±   1.6 ms    [User: 7.3 ms, System: 31.0 ms]
  Range (min … max):    39.5 ms …  47.0 ms    65 runs

Benchmark #2: ./symlink-all.sh
  Time (mean ± σ):      25.4 ms ±   1.4 ms    [User: 6.3 ms, System: 15.2 ms]
  Range (min … max):    23.3 ms …  30.5 ms    89 runs

Summary
  './symlink-all.sh' ran
    1.67 ± 0.11 times faster than './copy-all.sh'
```

For a smaller file count on the smaller file size: (10 files of 16KB each):

```
$ hyperfine ./copy-all.sh ./symlink-all.sh
Benchmark #1: ./copy-all.sh
  Time (mean ± σ):      19.3 ms ±   1.2 ms    [User: 5.8 ms, System: 9.6 ms]
  Range (min … max):    17.3 ms …  23.3 ms    125 runs

Benchmark #2: ./symlink-all.sh
  Time (mean ± σ):      17.0 ms ±   1.2 ms    [User: 5.6 ms, System: 7.8 ms]
  Range (min … max):    15.3 ms …  20.3 ms    128 runs

Summary
  './symlink-all.sh' ran
    1.13 ± 0.11 times faster than './copy-all.sh'
```

In all cases, symlinking is faster than copying.
In the case of large files, this difference is pretty stark!

I think the question becomes: is it worth the potential side effects to get this performance?

## Things Other Build Systems Do

### Nix / NixOS

Nix copies files into the isolated build directory.
When they're creating derivations from derivations they'll often symlink files between paths in `/nix/store` though (see [the `symlinkJoin` docs for how this works](https://github.com/NixOS/nixpkgs/blob/5165af0033eb17bc9668f21215833aa1eb203f01/pkgs/build-support/trivial-builders.nix#L273-L317))

### Bazel

[Bazel seems to use a combination of symlinks and copies](https://docs.bazel.build/versions/2.0.0/output_directories.html).
As of 2018 [they mentioned they were using symlinks in their isolater in a blog post about sandboxfs](https://blog.bazel.build/2018/04/13/preliminary-sandboxfs-support.html).

## Appendix A: Benchmark Data

### Generating Test Data

```
mkdir test
for i in $(seq 1 100); do
  dd if=/dev/urandom of=test/$i.zero count=1024 bs=1024
done
```

(Small files test used `count=16 bs=1024`)

### `copy-all.sh`

```sh
#!/usr/bin/env bash

TEMP="$(mktemp -d)"
finish() {
  rm -rf "$TEMP"
}
trap finish EXIT

cp test/* "$TEMP"
```

### `symlink-all.sh`

```sh
#!/usr/bin/env bash

TEMP="$(mktemp -d)"
finish() {
  rm -rf "$TEMP"
}
trap finish EXIT

ln -s test/* "$TEMP"
```
