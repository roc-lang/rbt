# ADR 010: What to do with logs?

**Problem:** when commands fail, you usually want to look at the process logs to see what went wrong.
You also might want to see logs as the system is building.
This is more complicated than just "ok, so stream them" because we plan to add remote builders.
How do we persist logs so you can look at them later, for example when a CI job fails?

It seems like the best solution would be to have both a log store and a log sink.
Specifically:

1. during evaluation of jobs, logs can be streamed.
   This would let us show them in the CLI or in a web UI.
2. Logs could be stored like job artifacts in the content-addressable store.

Part 1 can be implemented today, but part two needs a little bit more groundwork.
Specifically, how do you keep track of which logs came from which jobs?
You could store them with the artifacts, but that seems unreasonable in the case of failing jobs (since it would be unreasonable to store artifacts for a failing job.)

Instead, it seems like we need the concept of an invocation, which could keep track of which jobs it did and didn't build, along with log outputs.
Then, the invocation (along with metadata, logs, and anything else) could be stored in the content-addressable store.

That means that if you invoke a job remotely, the result could be streamed back to you, or you could download it for easy inspection.
To start with, the structure could look like:

```
some-invocation
└── logs
    └── some-job-hash
        ├── stderr.log
        └── stdout.log
```

... replacing `some-invocation` with the invocation hash, and `some-job-hash` with the job hash.

We'll surely fill this tree out with interesting things, so it makes sense to leave room for additional hierarchy.
