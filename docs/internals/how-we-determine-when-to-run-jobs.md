# How We Determine When to Run Jobs

rbt tries to avoid re-running jobs it has seen before.
To do this, we assume that builds are reproducible and keep track of which jobs produce which files.
If we already have the output files for some build configuration, we just reuse them instead of rebuilding!

## Caches

To manage rebuild logic, we keep several levels of caches.
Here's how they work:

### Level 1: Job Keys

`Job` definitions create a cache key by hashing their configuration.
That means we use all the information from the command, args, input file names, etc.
It's important that `Job` does not do any filesystem operations when calculating this key, since multiple jobs reading and hashing the same configuration file (for example) would be a lot of duplicated work.

### Level 2: Job Keys + Input File Hashes

At the start of an rbt build, we collect all the files for all jobs and deduplicate their paths.
For each path, we get metadata about the file and use it to look up the file's content hash in a persistent store.
If we don't have the hash, we calculate and store it using [BLAKE3](https://en.wikipedia.org/wiki/BLAKE_(hash_function)#BLAKE3).

Then, for each `Job`, we produce a final key by combining the input hashes of all the files and content-addressable store paths of input jobs (see below) with the job's base key.

### Level 3: Execution and the Output Store

We store all the output of builds in a [content-addressable store (CAS)](https://en.wikipedia.org/wiki/Content-addressable_storage).
As the final step before running a job, we check in a mapping between job+hash keys and store paths.
If we already have a store path, we assume that the build would produce the same output and skip running it.

When a build succeeds, we hash and store the content in the CAS store for use in the next run.

## A note on metadata vs hashing (and why we use both)

Older build systems (like Make) use only the last-modified time (herinafter mtime) of the file.
Newer build systems (like Shake) can be configured to use only hash.
We use both, and lots of other filesystem metadata besides.
Why is that?

First of all, mtime comparisons causes problems in unintuitive ways ([as documented by Avery Pennarun](https://apenwarr.ca/log/20181113)).
To sum up: you can't safely assume that mtime will only increase, or increase in a way that's always reliable to know if you need to rebuild.

You can get a pretty good idea by combining mtime with other information, though!
For example, we definitely need to rebuild if a file's length changed.
We look at more than just that, though: in addition to length and mtime (calculated on all systems), we also read the inode number, permissions, UID, and GID on Unix-family systems (e.g. Linux and macOS.)

"Add more metadata" is good enough to avoid the problem of not rebuilding when a file changes, but we can still get unnecessary rebuilds when the file metadata changes without the content changing (e.g. `touch -m some-file`.)
To avoid this (and be able to skip as many rebuilds as possible) we also hash all the files.
It would be unacceptably slow to recalculate hashes for file on every run, though, so we cache them according to a key derived from the metadata.

This means making a bit of a tradeoff on flexibility: we can't rely on builds reliably producing side effects (e.g. uploading a built artifact to some store.)
However, rbt tries to avoid uncontrolled side-effecting behavior in general, so this is OK for us!
