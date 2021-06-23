# ADR 001: Job Isolation

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
We can't rely on things like `chroot` or isolating the network.
We may be able to do *some* of these things, *some* of the time, but not when it conflicts with the goals above.

## Things Other Build Systems Do

### Don't Isolate At All

Many common build tools (make, shake, rake) do not isolate anything.
Processes inherit the environment and run in the working directory to create new files.

### Blaze Descendants

Build systems derived from Google's Blaze like Bazel, Please, and (I think) Buck do something similar to what we're proposing here.

- [Please documents that builds are "hermetic"](https://github.com/thought-machine/please#how-is-it-so-fast).
- [Bazel uses a FUSE module](https://docs.bazel.build/versions/main/sandboxing.html), [introduced in 2017](https://blog.bazel.build/2017/08/25/introducing-sandboxfs.html), and [improved in 2018](https://blog.bazel.build/2018/04/13/preliminary-sandboxfs-support.html).
- [Buck has some level of sandboxing](https://buck.build/files-and-dirs/buckconfig.html#sandbox).
  The docs there make me think it's using the macOS `sandbox` command, although I haven't looked at the implementation.

### Nix / NixOS

Nix seems to copy files to a temporary directory and remove the environment, the way I'm suggesting we do here.
When run on NixOS, Nix also seems to have some way to restrict network access.
Further research may be helpful, as the mechanism seems pretty close to what we want.

## Options We Have in Different Operating Systems

### macOS

macOS has `sandbox` and `sandboxd` to restrict access to various system calls.
Running `man sandbox-exec` suggests that at least that command is deprecated.
I need to do more research here.

We can also use `dtrace`, if we're OK with asking people to turn off SIP (we're not.)

macOS also has `chroot` if we're OK taking the time to set up the entire root (which may be too much.)

### Linux, BSD, etc

These systems can use `fstrace` can see what files a process is reading and writing.
Using that, we can at least warn if someone accesses or writes a file outside the allowed list.
I don't remember off the top of my head how much it slows down the process, though.

BSD has [`jail`s](https://en.wikipedia.org/wiki/FreeBSD_jail), which would maybe be helpful.

Linux also has `chroot` if we're OK taking the time to set up the entire root (which may be too much.)

### Windows

I just haven't the foggiest, and as I'm typing this don't have time to research either.

### Virtualization

It may be possible to isolate using some level of virtualization, either with Docker or full-strength VMs.
This would be really reproducible, although at a high cost to speed.

## Our First Pass

So to start off with, we're going to create a baseline level of isolation by:

- Removing the contents of the environment.
  That means the environment will be explicitly and exactly specified, not inherting anything from the calling environment.
  This avoids problems where things from `LOCALE` or `USER` make their way into the build output.
  However, we recognize that this will probably break a lot of build tools, and we plan on handing the most common cases (like `HOME` and `PATH`) in a friendly way.
- Explicitly tracking input and output files.
  Since we can't rely on using `ftrace` or `strace` or similar across platforms, we can get this by running files in a temporary directory where we explicitly copy input files into and output files out of.
  We recognize that this may cause some build slowdown, but we're going to start here and see if there's are ways to speed it up or more complex/clever approaches if and when it becomes an issue.
