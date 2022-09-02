# How We Determine When to Run Jobs

rbt tries to avoid unnecessary re-runs of jobs whenever possible using a layered approach.

Here's a quick summary about how the process works.
Afterwards, we'll go into more detail about each stage.

1. Each job produces a cache key from its fields (command, args, environment, etc.)
2. rbt's coordinator module adds information about input files and jobs to the key.
3. If we have an output path for the combined key, we skip the build.
4. Otherwise, we run the build and store a content-addressed hash.

## Job Hashes

`Job` definitions create the first level of cache key by hashing the information it has direct access to: things like command and args or input file names.

Wherever possible, these keys avoid ordering or duplication: for example, the input file set `["a", "b"]` is not going to produce a different build just by being `["b", "a"]` or `["a", "b", "a"]`, so we'll treat that field as a set.
Note that not all these validations are possible to enforce in the Roc API, but for those that will become possible (e.g. accepting sets directly instead of lists) we plan to break the API in releases before 1.0.0.

It's important that `Job` does not do any filesystem operations when calculating this key.
Multiple jobs reading and hashing the same configuration file (for example) will create a lot of unnecessary work.

We do, however, include the paths to inputs.
For example, what if we include `["a", "b"]` but then "b" later moves to "c" but has the same contents?
If we did not include the path change, the cache key would not change and the build might not be re-run.
(Although re-run might not be *strictly* necessary in this case, but rbt doesn't have enough information about the toolchains in your build to know this!)

## Input Hashes

Once we have a base hash for a `Job`, rbt collects all the files for all jobs, deduplicates paths, and fetches content hashes.

We avoid some work here, too, by looking first at metadata about the files before calculating their hashes.

Once we have that key, we look up the file hash in a persistent map.
If we have the hash, great!
We add it to the execution key and move on.

Otherwise, we calculate a hash of the file's contents (currently using [BLAKE3](https://en.wikipedia.org/wiki/BLAKE_(hash_function)#BLAKE3)) and store it in the persistent map.

<!-- aspirational as of 2022-09-02 -->
We also add the content-address store paths of the jobs the current job depends on at this point.
(See below for how we define those paths.)

### A note on metadata vs hashing (and why we use both)

Older build systems (like Make) use only the last-modified time (herinafter mtime) of the file.
Newer build systems (like Shake) can be configured to use only hash.
We use both, and lots of other filesystem metadata besides.
Why is that?

First of all, mtime comparisons causes problems in unintuitive ways ([as documented by Avery Pennarun](https://apenwarr.ca/log/20181113)).
To sum up: you can't safely assume that mtime will only increase, or increase in a way that's always reliable to know if you need to rebuild.

You can get a pretty good idea by combining mtime with other information, though!
For example, if a file's length changed then you definitely need to rebuild.
We look at more than just that, though: in addition to length and mtime (calculated on all systems), we also read the inode number, permissions, UID, and GID on Unix-family systems (e.g. Linux and macOS.)

"Add more metadata" is good enough to avoid the problem of not rebuilding when a file changes, but we can still get unnecessary rebuilds when the file metadata changes without the content changing (e.g. `touch -m some-file`.)
To avoid this and be able to skip as many rebuilds as possible, we also hash all the files.
It would be unacceptably slow to recalculate hashes for every file, though, so we cache them according to a key derived from the metadata.

This means making a bit of a tradeoff on flexibility, however: we can't rely on builds reliably producing side effects (e.g. uploading a built artifact to some store.)
However, rbt tries to avoid uncontrolled side-effecty behavior in general, so this is OK for us!

## Execution and the Output Store

We store all the output of builds in a [content-addressed store (CAS)](https://en.wikipedia.org/wiki/Content-addressable_storage).
As the final step before running a job, we check in a mapping between job keys and CAS.
If we already have a store path, we assume that the build would produce the same output and skip running it.

If we don't have a store path for the current hash, we run the build and hash the content to get a CAS path, store it, and move on to the next job!
