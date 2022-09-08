# ADR 007: What's a Cache?

Many language ecosystems have some concept of a mutable, ephemeral cache which the compiler or other tools can use to speed up future tasks.
rbt will manage these caches for you.
The full options will look something like this:

```roc
elmStuff : Cache
elmStuff =
    cache
        {
            name: "elm-stuff",
            persistAt: "/elm-stuff",
            parallelism: ThreadSafe,
        }
```

As a quick run-down:

- `name` gives a name to distinguish between multiple caches in CLI output, logs, etc
- `persistedAt` indicates that the cache will be persisted outside the build context.
  It has the same meaning that it does in jobs (see [ADR #5](./005-jobs.md))
- `parallelism` lets rbt know whether it needs to manage a lock for this resource (the default if not specified) or if the cache can be shared between multiple jobs simultaneously.
  For example, a compiler might manage files in the cache with `flock` to make sure multiple instances of the compiler did not overwrite each other's work.

A job will use a cache like this:

```roc
app : Job
app =
    job
        { 
            command: exec "elm" [ "make", "--output", "app.js", "src/Main.elm" ],
            inputFiles: [ "elm.json", "src/Main.elm" ],
            outputFiles: [ "app.js" ],
            caches: {: "/elm-stuff" => elmStuff :}
        }
```

## Caches Are Mutable and Ephemeral

We assume that if we delete a cache, nothing bad or different will happen (other than the build potentially taking longer.)
That means they're well-suited for compiler artifacts, but could potentially cause trouble for other uses.

Specifically, having "this is shared mutable state" is a big hole to punch in the API, and it can easily be used for other things.
We should be very careful to communicate that, whether it's through naming or documentation.

Specifically, I'm a little worried that people will see this and think "ah, a *download* cache."
It'd totally work for that, but if we're downloading stuff it'd be far better to describe the download using a first-class API construct so that rbt can track it (and potentially share it between systems via a build server) to reduce the network dependency.
That'll have to be a future ADR.

## How Does This Affect Content Hashing?

Because caches are mutable and ephemeral, they don't affect content hashing at all.
This definitely has the potential to introduce irreproducibility into builds, so a "production" build will introduce a fresh cache to each job.
(Which, as a side note, hopefully reduces the size of the loophole of caches being used for things outside their intended purpose.)
