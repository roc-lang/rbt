# ADR 006: What's a Job?

We want to be able to specify builds in Roc (I mean, it's right there in the name!)

To do that, we need to have a good idea about the kinds of data we're working with and how they relate to one another.

## Jobs

Jobs make up the basic units of work and output in rbt.
They depend on the files in the working tree (see [ADR #5: Dynamic Dependencies](./005-dynamic-dependencies.md)) as well as the outputs of other jobs.
This creates a dependency graph that rbt walks to build.

### Commands

Commands are the only truly required part of a job.
Without them, nothing else makes sense!
A trivial job looks like this:

```roc
hello : Job
hello =
    job { command: exec echo [ "Hello, World!" ] }
```

Of course, there's not much point to this job since it doesn't produce any output.
We'll get to that (and the definition of `echo`) below.

Of course, since Roc is a full programming language, you can define your own helpers easily.
Let's make one that executes an inline shell script to keep the rest of this document tidy:

```roc
execShellScript : String -> Command
execShellScript = \script ->
    exec sh [ "-c", script ]
```

There are handful of minor things to remember about commands:

- We only consider a job successful if it exits with a `0` exit code.
- We will keep stdout and stderr around for inspection, but won't attempt to process them in any way.
  If you want to add ANSI escape codes for colors and whatnot, knock yourself out.
- Commands should assume they're being run non-interactively.
  We may not enforce this immediately but may need to in order for remote building to work.
- We won't provide any information in stdin.
  If your command needs to read a file, it should specify it in inputs (read on!)

### Outputs

Let's make `hello` slightly more useful by producing a file:

```roc
hello : Job
hello =
    job
        {
            command: execShellScript "echo Hello World > hello",
            outputs: [ "hello" ],
        }
```

Specifying `hello` here will mean that `rbt` will expect there to be a file named `hello` in the working directory after the job completes.

There are a handful of minor things we want to remember about outputs:

- If a job completes successfully, the outputs will be exactly the set of files specified.
  No extra files will be copied, and the job will fail if any are missing.
- It's fine for commands to not output anything.
  Linters and tests, for example, can control success or failure based on command exit code.
  There just won't be any outputs for other jobs to read in this case!
- rbt will cache job output internally (instead of discarding results from intermediate jobs) so we can inspect or share the outputs in future builds.
  (Of course, inspection implies a way to ask for a specific job to be built, either from the command line or an API.
  This ADR doesn't define that, but we'll have to soon.)

---

- [ ] Should we require output files be written to a special directory instead?
      Nix does this with `$out`.
      It works just fine, and removes the need to specify outputs up front.

- [ ] Should we allow specifying directories as outputs?
      Feels globby, so it might have some of the same concerns as globs in inputs.

#### Output Persistence

Builds sometimes need to put files back into source directories.
For example, a job could generate an API client, the files of which would be necessary for editor tooling to provide autocompletion or typechecking.
So, in addition to specifying outputs, jobs will be able to specify where their outputs will be persisted.

You might use persistence like this:

```roc
hello : Job
hello =
    job
        {
            tools: [ openapiGenerator ]
            command: exec "openapi-generator-cli" [ "generate", "-i", "spec.json", "-o", "api-client", "-t", "elm" ],
            outputs: [ "api-client" ],
            persistAt: [ "/src/api-client" ],
        }
```

### Inputs

As discussed in [ADR #5](./005-dynamic-dependencies.md), jobs can have two kinds of basic dependencies: a list of file paths known at compile time, and a set of dynamic discovery scripts.
Jobs add a third kind of dependency here: jobs can depend on other jobs, which specifies the complete build graph.

You might specify a job with file inputs like this:

```roc
app : Job
app =
    job
        {
           command: exec elm [ "make", "--output=app.js", "src/Main.elm" ],
           inputFiles: [ "elm.json", "src/Main.elm" ],
           outputFiles: [ "app.js" ],
        }
```

And one which builds on another job like this:

```
uglifiedApp : Job
uglifiedApp =
    job
        {
            tools: [ uglifyjs ],
            command: execShellScript "uglifyjs app.js --compress | uglifyjs --mangle --output app.min.js",
            inputJobs: [ app ],
            outputFiles: [ "app.min.js" ],
        }
```

(n.b. specifying `inputJobs` separately from `inputFiles` and the yet-to-be-named discovery argument seems weird, right?
But consider this: if we put these all in one argument, e.g. `inputs`, we would either have to wrap all the strings in some special function or lose dependency information from jobs.
By specifying them separately, we get both nice notation and make inspecting the graph easier.)

Some things we should remember about inputs:

- Jobs must specify their complete set of inputs, and a job will be rebuilt any time any of its inputs change.
- Jobs will not be able to see any files outside the inputs they specify (see [ADR #1](./001-job-isolation-targets.md).)
- Caches (described in [ADR #7](./007-caches.md)) are also *technically* inputs, but not in the rebuild sense.
  We expect that an empty cache will not cause a build to fail, so a cache changing will not trigger a rebuild.

### Environment

Of course, commands often need environment variables to work properly, so you can specify those:

```roc
hello : Job
hello =
  job
      {
          command: execShellScript "echo $GREETING",
          environment: {:
            "GREETING" => "Hello, World!",
          :},
      }
```

### Tools

A tool is just a binary that rbt knows about.
We add specified tools to the `PATH` of the build environment so jobs can use them.

There are a couple of ways to source tools.
The simplest is to assume the tool already exists on the system:

```roc
gunzip : Tool
gunzip = systemTool "gunzip"
```

This would search through the host system's `PATH` to find a `gunzip` binary.

You can also use tools to source other tools:

```roc
nixShell : Tool
nixShell = systemTool "nix-shell"


curlBinary : Job
curlBinary =
  job
      {
          tools: [ nixShell ],
          command: exec "nix-shell" [ "-p", "curl", "--run", "ln -s $(which curl) curl" ],
          outputs: [ "curl" ],
      }


curl : Tool
curl = 
  tool curlBinary "curl"
```

(Note that we may want to eventually make an easier way to source tools from large package ecosystems like Nix or Homebrew, but for now we can use jobs to do whatever we want!)

And, of course, we can also source tools from the internet:

```roc
elm : Tool
elm =
  job
      {
          tools: [ curl, gunzip ],
          command: execShellScript "curl -L https://github.com/elm/compiler/releases/download/0.19.1/\(filename) | gunzip > elm && chmod +x elm",
          outputs: [ "elm" ],
      }
      |> tool "elm"
```

(Note that eventually we should have built-in way to download things that does checksumming and more caching.
That's for a future ADR!)

Finally, we provide a way to wrap binaries in the environment they need so they can be used in jobs without any further wrapping:

```roc
npm : Tool
npm = systemTool "npm" 


node : Tool
node = systemTool "node"


nodeModules : Job
nodeModules =
    job
        {
            command: exec npm [ "install" ],
            inputs: [ "package.json", "package-lock.json" ],
            outputs: [ "node_modules" ],
        }


uglifyjs : Tool
uglifyjs
    customTool
        {
            path: "\(nodeModules.outputs)/node_modules/.bin/uglifyjs",
            tools: [ node ],
            environment: {:
                "NODE_PATH" => "\(nodeModules.outputs)/node_modules"
            :}
        }
```

- [ ] Surely there is a better way to do this.
      Needs more thought.

### CPU Hinting

rbt uses job parameters to jobs to construct a build graph, which it will then walk in parallel wherever possible.
This means that we can do a better job scheduling tasks when the tasks themselves are smaller—think compressing a single image instead of a whole directory of them.

However, many compilers (like Zig, Rust/Cargo, Elm, Haskell, and Roc itself) have a monolithic compilation process where they take an entrypoint and manage compiling all the dependencies themselves.
This is great both for programmers and compiler authors: it unlocks optimization opportunities and in many cases does away with the need for a separate build tool.

Unfortunately, it's harder for a generic build tool like rbt to deal with those kinds of processes in a build step.
The big question here: should we schedule work on other cores while the big process is running?
We don't want to cause the CPU to do too much context switching!

To fix this, jobs can give rbt enough information to do the right thing by specifying **CPU Hints**.
A job can either say it takes a single core or that it will saturate all available cores.

A future ADR (or just the implementation) will determine how we deal with CPU hints, but one possibility is to avoid starting new work when there's a saturating job running.
Another would be to use some concept of CPU reservation to avoid starting a saturating job before it held the lock on all cores (of course, we'd have to be careful to avoid creating deadlocks here!)

Anyway, here's how you might use this:

```roc
binary : Job
binary =
    job
        {
            tools: [ cargo ],
            command: exec "cargo" [ "build", "--release" ],
            outputs: [ "target/release/binary-name" ],
            cpuHint: SaturatesCpus,
        }
```

### Fixers

Code formatters and some linters can automatically fix problems in source code.
Jobs can specify a flag to make this explicitly OK.
It might look like this:

```roc
elmFormat : Job
elmFormat =
    job
        {
            tools: [ elmFormat ],
            command: exec "elm-format" [ "--yes" ],
            inputs: [ "src/Main.elm", "src/OtherModule.elm" ],
            modifiesInputs: True,
        }
```
