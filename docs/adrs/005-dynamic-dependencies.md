# ADR 005: Dynamic Dependencies

## Decision

For now, rbt will allow both static (read: known at compile time) and dynamic (read: discovered at runtime) dependencies.

Static dependencies will be a list of strings (or equivalent.)

Dynamic dependencies will be discovered via Roc scripts running on a custom platform that limits I/O.
(Read on for more details.)

## How We Got Here

The big question here: should rbt require that the files in the build graph be completely specified ahead of time, or should we allow dependencies to be generated dynamically?
This is at odds with a goal of having quick builds, since any level of dynamic dependencies requires having a dependency discovery process, and that eats up time before we can actually start running build commands.
That said, it turns out that dynamic dependencies are at least often useful, and apparently sometimes absolutely essential (e.g. building Fortran?)

I know of four ways to slice this:

1. Discover dependencies while walking the build graph (like Shake and Redo)
   These implementations tend to have the build API which can declare additional dependencies and run commands in the same job.

2. Pause for discovery while building (like Make and Ninja)
   The process here looks like: when you get to a dynamic dependency, run a script and then pretend like you had that knowledge all along.
   On the next run, examine the dependencies of the dependency script to decide whether or not to re-run it.
   (Make does some magical/hard-to-explain things here!)

3. Discover dependencies and build in separate phases.
   I couldn't find any existing systems that do this!

4. Don't allow dynamic dependencies (Buck, Bazel, and kinda Nix and Meson.)
   These implementations have long lists of dependency files.
   Sometimes they allow globs or directories as inputs, sometimes they don't.

Ok, so let's try rule some of these out:

- We already decided (outside of ADRs) to avoid #1 because doing that means we give up having a statically-inspectable build graph.
- We can rule out #2 for the same reason, although we may want to borrow the caching mechanism!
- We'd prefer to avoid #4, purely for ease of use: it can be annoying to have to specify an import in both your source code and build system.

That means we're left with #3, but nobody chooses that for some reason.
So let's try and discover why that is.
If there's a reason to avoid it, let's avoid it, but otherwise it seems promising!

### Determining the Build Graph

A job can have any number of dependencies, with a mix of known-at-compile-time and unknown-at-compile-time.
The known ones are easy: they're just links in the graph, and we don't need to discuss them further.

The dynamic ones are a little harder.
For example, what if we say that we just run a script that digs around in the filesystem and returns a list of dependencies?

Seems fine at first, but it's actually a trap!

1. **What about dependencies of the discovery script?**
   Say we need to compile a Rust program: how do we discover `cargo`?
   Do we need to have a separate two-phase discovery/execution step to make the primary build graph?
   And that's bound to be true further down as well, needing a third level to build the second, a fourth to build the third, and so on.
   At that point, it might actually be a simpler mental model to get rid of the phases altogether and discover dependencies while building like Shake and Redo!

2. **How do we know when to re-run?**
   Assuming we've got our discovery dependencies sorted out, that's easy: just run whenever they or the input files for the job change.
   It's harder if we define discovery in a function, because we don't have an easy way to see if a function changed in Roc.

3. **What about I/O?**
   Assuming we solve the problems above, these scripts can do pretty much whatever they like on the filesystem.
   We would communicate that discovery scripts should only ever read from disk, but because we can't guarantee that happens folks would definitely write to the disk, do network stuff, et cetera.
   (Fortunately this problem is mostly true of scripts.
   A Roc API could restrict options in a useful way!)

4. **What about performance?**
   Specifically, we're allowing arbitrary computation and disk I/O in a place we care a lot about performance.
   Hopefully compiling dependencies and executing scripts is fast, but we have no guarantee.

However, we can address all of these problems by running Roc discovery scripts in a custom platform:

1. **What about dependencies of the discovery script?**
   We don't need to worry about external dependencies because we'll bundle what we need into the platform.

2. **How do we know when to re-run?**
   The Roc script will be a single file.
   When that file or any of the inputs change, we'll re-run it.

3. **What about I/O?**
   We can limit I/O to either nothing at all or a tiny subset of the possible operations (e.g. no writes, no directory scans.)

4. **What about performance?**
   This one is much harder to dismiss: you can still write non-terminating programs in Roc.
   However, we're on much more solid ground here!
   Roc is designed to both compile and run very quickly, so we're unlikely to see a ton of platform overhead.
   If we're goint to allow arbitrary computation, this is a pretty reasonable way to do it.

The details of the platform will still need to be hammered out, but the scripts will look basically like pure functions with metadata about what files they care about.
Rbt will read those files and pass the script the bytes, and the script passes a list of dependency paths back to rbt.

## Why Can't We Just…

### … Specify Globs and Walk the Source Tree?

I benchmarked globbing, and **it adds 100–1,200ms of overhead before we can start building**.
That's too much to commit to right now, so we're going to try and do without globs until we absolutely cannot get around it.

I used the [`ignore`](https://crates.io/crates/ignore) crate (used by `rg`, `fd`, etc), which has options for both parallelism and applying gitignores, and tested on two repos:

- [NixOS/nixpkgs](https://github.com/NixOS/nixpkgs) (45,744 files, fresh checkout)
- the NoRedInk monorepo (12,408 files checked in, 163,289 total.)

In four configurations: single-threaded and parallel, and ignoring git-ignored files and not.

| Configuration              | NoRedInk monorepo   | nixpkgs (45,744 files) |
|----------------------------|-:------------------:|-----------------------:|
| parallel, all files        | 795.4 ms ±  20.6 ms | 603.1 ms ±  20.1 ms    |
| parallel, gitignore        | 110.1 ms ±   5.2 ms | 748.6 ms ±  11.8 ms    |
| single-threaded, all files |  1.256 s ±  0.022 s | 928.5 ms ±  28.1 ms    |
| single-threaded, gitignore | 184.0 ms ±   8.1 ms |  1.249 s ±  0.025 s    |

Potential issues with this:

- These were pretty casually run.
  I didn't turn off CPU throttling or anything like that.
  However, it's safe to assume that people running rbt will not be turning off their music player, instant messenger, etc just to do a build either!
- `ignore` could be the wrong library for us to compare with.
  For example, it could be doing extra work we don't need, or have parallelism bugs I don't know about.
  I don't think that matters here because the goal was to get a ballpark of how much time we'd take on startup for a medium-to-large project.
  (For the record, I'd call a small project 0–1,000 files, a medium size project 1,000–100,000 files, and a large project anything above 100,000)

### … Run a Daemon to Amortize the Blobbing Overhead?

Running a daemon would make our startup overhead costs way more manageable since we could match globby paths over and over during the daemon's lifetime with a file watcher.
Adding a daemon is probably in rbt's future, but we want to make sure that a cold boot is as fast as possible first.

## Things Other People Have Done

### Meson

Meson [does not allow you to specify files by glob](https://mesonbuild.com/FAQ.html#why-cant-i-specify-target-files-with-a-wildcard) (their term is wildcard.)
They note that a no-op in a source tree of 10,000 files is pretty expensive if you have to glob match (which implies scanning the filesystem.)

### Shake

Shake discovers the build graph as it builds and allows glob matches.
This frustrates some forms of static analysis, as discussed above.

Neil Mitchell (author of Shake) [asserts that dynamic dependencies are a requirement of a build system at scale](https://neilmitchell.blogspot.com/2021/09/reflecting-on-shake-build-system.html):

> The most important thing Shake got right was adding monadic/dynamic
> dependencies. Most build systems start with a static graph, and then, realising
> that can't express the real world, start hacking in an unprincipled manner. The
> resulting system becomes a bunch of special cases. Shake embraced dynamic
> dependencies. That makes some things harder (no static cycle detection,
> less obvious parallelism, must store dependency edges), but all those make
> Shake itself harder to write, while dynamic dependencies make Shake easier
> to use. I hope that eventually *all* build systems gain dynamic dependencies.

### Redo

Redo ([Daniel J. Bernstein's writing](http://cr.yp.to/redo.html), [apenwarr's implementation](https://github.com/apenwarr/redo)) works similar to shake, but works all in shell-scripts.
A simple build script might look like:

```sh
redo-ifchange cc dhcpd.deps
redo-ifchange `cat dhcpd.deps`
./cc -o dhcpd `cat dhcpd.deps`
```

### Make

If you ask Make to [include a file and also provide a way to build that file](https://www.gnu.org/software/make/manual/make.html#Include), Make will build the file, add the new rules, and proceed as if it knew them all along.

### Ninja

[Ninja](https://ninja-build.org) allows both [dynamic dependencies](https://ninja-build.org/manual.html#ref_dyndep) (which work similar to Make's) and [header dependencies](https://ninja-build.org/manual.html#ref_headers) (which take advantage of compilers that can output dependency resolution based on their internal search schemes.)
