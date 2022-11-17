# ADR 011: Capture stdout/stderr

Problem: sometimes we want to capture the outputs of a command to a file.
For doing this, we currently have to use a shell, which means incorporating the environment of the shell in said output.
This can result in unexpected states, e.g. shell-set environment variables, as in [PR #83](https://github.com/roc-lang/rbt/pull/83).
Besides this, this also creates a dependency on a shell environment. This means having at least two subprocesses, which may slow down the job.
Not using a shell also allows better ergonomics, as we don't have to write a shell script for each job. This also means that the user can easily ignore stderr if they want to.

The solve this problem, we're gonna provide a way to capture a command's stdout and/or stderr.

## Implementation

The implementation we're going for is adding `stdout` and `stderr` optional fields to `Job`.
These fields would be represented by either:
  * `Stream`: in which we'll not capture the results; or
  * `Capture Str`: which should receive a filepath, and will store the results of the command to said path. The path should also be a relative path within the output directory.

With this implementation, `Job` will look like:
```elixir
Job := [
    Job
        {
           ...
           stdout : [Stream, Capture Str],
           stderr : [Stream, Capture Str],
           ...
        },
]
```
 
