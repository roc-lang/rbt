# ADR 001: The Lowest Common Denominator of Job Isolation

When you're running a build, there are a lot of things that can interfere with reproducibility:

- Environment variables like `LOCALE`, `LANG`, or even `PATH`.
- Untracked files like caches and config files in your home or XDG config directories.
- Untracked-but-cached files in the local directory.
- System time getting swept up in the build output.
- Remote sources changing unexpectedly
- ... and lots more!

However, we find reproducibility helpful!
It lets us do things like addressing files by their content to avoid rebuilds and share results across machines.
So, we should work on reducing sources of irreproducibility wherever possible.

That said, for rbt, we want to stay within these constraints:

- We should be able to work on whatever operating systems you can use Roc on (macOS, Windows, Linux, etc.)
- We shouldn't require elevated privileges to run a build.

That means we're probably limited to doing things like copying files and spawning subprocesses.
We can't rely on things like `chroot` or isolating the network, since they usually require either root privileges or are not available on all platforms.
We may be able to do *some* of these things, *some* of the time, but not when it conflicts with the goals above.

## Things Other Build Systems Do

### Don't Isolate At All

Many common build tools (make, shake, rake, redo) do not isolate anything.
Processes inherit the environment and run in the working directory to create new files.

### Blaze Descendants

Build systems derived from Google's Blaze like Bazel, Please, and Buck isolate the environment and file system, at least a little.

[Please documents that builds are "hermetic"](https://github.com/thought-machine/please#how-is-it-so-fast).
It looks like they [symlink files into a temporary directory](https://github.com/thought-machine/please/blob/6185d5d4f0179fba68d89c6eac294ab7d862d898/src/core/utils.go#L332-L344).

[Bazel uses a FUSE module](https://docs.bazel.build/versions/main/sandboxing.html), [introduced in 2017](https://blog.bazel.build/2017/08/25/introducing-sandboxfs.html), and [improved in 2018](https://blog.bazel.build/2018/04/13/preliminary-sandboxfs-support.html).
In that 2017 blog post, they mention that they also use symlinks in a temporary directory (but that it can cause problems if build tools try to chase the link to use the "real" path.)

[Buck has some level of sandboxing](https://buck.build/files-and-dirs/buckconfig.html#sandbox).
The docs imply that they're using the macOS `sandbox` command (because the name and the fact that it's only Darwin) and [it looks like their implementation backs that up](https://github.com/facebook/buck/blob/0acbcbbed7274ba654b64d97414b28183649e51a/src/com/facebook/buck/sandbox/darwin/DarwinSandbox.java).
There's [a lot of indirection in their general sandbox implementation](https://github.com/facebook/buck/tree/0acbcbbed7274ba654b64d97414b28183649e51a/src/com/facebook/buck/sandbox), and it looks like Darwin/sandbox may be the only real sandboxing implementation (other than the "don't sandbox" implementation.)

### Nix / NixOS

Nix isolates builds (they call it sandboxing) with a few different mechanisms.
It looks like they create a temporary working directory no matter what platform you're using.

On Linux, they default to sandboxing, which requires root privelegs (and so defaults to `false` on macOS.)
The [docs on the `sandbox` option](https://nixos.org/manual/nix/stable/#conf-sandbox) say:

> If set to `true` builds will be performed in a *sandboxed* environment, i.e., they’re isolated from the normal file system hierarchy and will only see their dependencies in the Nix store, the temporary build directory, private versions of /proc, /dev, /dev/shm and /dev/pts (on Linux), and the paths configured with the sandbox-paths option.
> [...]
> In addition, on Linux, builds run in private PID, mount, network, IPC and UTS namespaces to isolate them from other processes in the system (except that fixed-output derivations do not run in private network namespace to ensure they can access the network).

## Options We Have in Different Operating Systems

### macOS

macOS has `sandbox` and `sandboxd` to restrict access to various system calls.
`man sandbox-exec` suggests that the command is deprecated, and that developers should use [the App Sandbox features in new apps](https://developer.apple.com/library/archive/documentation/Security/Conceptual/AppSandboxDesignGuide/AboutAppSandbox/AboutAppSandbox.html), which looks like it's exclusively focused on the kinds of apps you get in app stores... so, not really the problem we have.
Some further research tells me that [Apple might have made this publicly deprecated because the configuration language is undocumented and they want to focus on App Sandboxing](https://developer.apple.com/forums/thread/661939) and that [Bazel does indeed use this, or at least did at one point](https://jmmv.dev/2019/11/macos-sandbox-exec.html).

We can also use `dtrace`, if we're OK with asking people to turn off SIP (we're not.)

macOS also has `chroot` if we're OK taking the time to set up the entire root (which may be too much.)

### Linux, BSD, etc

These systems can use `fstrace` can see what files a process is reading and writing.
Using that, we can at least warn if someone accesses or writes a file outside the allowed list.
I don't remember off the top of my head how much it slows down the process, though.

BSD has [`jail`s](https://en.wikipedia.org/wiki/FreeBSD_jail), which would maybe be helpful.

Linux also has `chroot` if we're OK taking the time to set up the entire root (which may be too much.)

### Windows

It seems like there's no one good answer to this on Windows. [Someone asked for this on StackOverflow and got extensive answers](https://stackoverflow.com/questions/135802/put-a-process-in-a-sandbox-where-it-can-do-least-harm), all of which are basically the shrug emoji.

That said, [Windows Containers exist and have several different isolation modes](https://docs.microsoft.com/en-us/virtualization/windowscontainers/manage-containers/hyperv-container).
I think if we did that we'd want to do a general Docker- or container-based isolator instead of a OS-specific one, but just something to keep in mind.

### Virtualization

It may be possible to isolate using some level of virtualization, either with Docker or full-strength VMs.
This would be really reproducible across supported platforms, and offer a consistent permissions model, but potentially have a high cost to speed.

## The First Pass

Our current code creates a baseline level of isolation by:

- Removing the contents of the environment.
  That means the environment will be explicitly and exactly specified, not inherting anything from the calling environment.
  This avoids problems where things from `LOCALE` or `USER` make their way into the build output.
  However, we recognize that this will probably break a lot of build tools, and we plan on handing the most common cases (like `HOME` and `PATH`) in a friendly way.

- Explicitly tracking input and output files.
  Since we can't rely on using `ftrace` or `strace` or similar across platforms, we can get this by running files in a temporary directory where we explicitly copy input files into and output files out of.
  We recognize that this may cause some build slowdown, but we're going to start here and see if there's are ways to speed it up or more complex/clever approaches if and when it becomes an issue.

## The Next Pass(es)

It seems like at least the Blaze descendants take the approach of making a bunch of symlinks instead of copying files.
This trades safety for speed (but potentially a lot of speed!)
In large builds, or in builds with many dependencies, it probably makes sense.
Switching to that is probably a reasonable next step.

It also seems reasonable to define isolaters for specific platforms, details of which will be in future ADRs.

We also need to define how we'll load programs.
If we're destroying `PATH` we'll need to specify our own load system somehow.

Finally, we need to work out what we'll do with `HOME` and other common environment variables.

All those are issues for further ADRs!
