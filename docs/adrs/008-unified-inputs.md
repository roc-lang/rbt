# ADR 008: Unified Inputs

To build, we need to get files from the filesystem and from inter-job dependencies.
The API we have so far has just been a bunch of different fields on `Job`:

- `inputs : List Job` for jobs that the `Job` depends on
- `inputFiles : List Str` for files from the project
- `inputPatterns : ???` (speculative) for discoverable files
- `dynamicInputs : ???` (speculative) for [dynamic dependencies](./005-dynamic-dependencies.md)

But this design has a couple problems:

- **It's too big.**
  If these fields are optional, there's a feeling of hidden functionality in the API.
  On the other hand, if they're *not* optional, it's a lot of empty lists to specify for a simple build.

- **It allows bad assumptions about file existence.**
  if a build depends on some file from a `Job`, it doesn't have to say so explicitly and so we can't check it and provide good guidance if the file is missing.

- **It doesn't say where files come from.**
  Since everything gets put into the filesystem, it's also not clear where a missing file was supposed to have come from.

- **It allows collisions.**
  If you need a file from the filesystem named the same thing as a file from a `Job`, there's no way to specify which takes precedence.

This ADR proposes that we unify these fields into `inputs : List Input` (really `Set Input` as soon as possible.)

`Input` is defined roughly like this:

```coffeescript
Input := [FromSource InputPath, FromJob { job : Job, files : List InputPath }]

# Add the given file to the job's workspace (the working directory where the
# command is called.)
file : Str -> Input
file = \inputPath -> FromSource (path inputPath)

# Set up a file from the project source (first argument) in exactly the location
# you need it within the workspace (second argument.)
renamedFile : Str, Str -> Input
renamedFile = \from, to -> FromSource (rename from to)

# Put all of the outputs of the given job in the current job's workspace.
#
# This is the easiest way to source files from other jobs. That said, if you have
# multiple jobs with the same file this can cause conflicts. In that case, use
# `someOutputsOf` to tell me exactly what you want.
allOutputsOf : Job -> Input
allOutputsOf = \Job config -> FromJob { job: Job config, files: List.map path job.outputs }

# Use this function to set up a the files from another job exactly the way you
# want in your workspace. You can pick exatly the files you need like
# `someOutputsOf someJob [file "the-one-I-want"]` or put them exactly in the
# locations you need like `someOutputsOf someJob [rename
# "path-that-doesnt-work-for-me" "a/better/path"]`.
someOutputsOf : Job, List InputPath -> Input
someOutputsOf = \job, files -> FromJob { job, files }

InputPath := [Path Str, RenamePath { from : Str, to : Str }]

# Use the path exactly as given, with no shenanigans.
path : Str -> InputPath
path = \str -> Path str

# Make the specified input path (first argument) available at a different
# location in the workspace (second argument.)
rename : InputPath, Str -> InputPath
rename = \from, to -> ...
```

Usage might look like this:

```coffeescript
completedGreeting : Job
completedGreeting =
    job {
        command: exec (systemTool "bash") [
            "-c",
            "printf '%s, %s!\n' "$(cat greeting.txt)" "$(cat subject.txt)" > completedGreeting.txt",
        ],
        inputs: [
            allOutputsOf subject,
            someOutputsOf greeting [rename (file "englishGreeting.txt") "greeting.txt"],
        ],
        outputs: ["completedGreeting.txt"]
    }

subject : Job # has `subject.txt` in its outputs

greeting : Job # has at least `englishGreeting.txt` in its outputs
```

This can be extended with filesystem matchers (our version of glob matches) and dynamic dependency discoverers later, without breaking the API.
(As a matter of fact, matchers might take over `FromSource`'s job eventually!)

## Benefits

This API:

- Lets us specify exactly how jobs want the filesystem to be set up before they run
- Lets us warn about conflicts between different files.
- Lets us see exactly where we're trying to source files from.
  This means we can see when a file would not exist and warn about that.
  (`allOutputsOf` defeats this a bit since it automatically depends on whatever files are available, but overall I think it's a win.)
- Opens up new optimization opportunities for caching: if we know that we only depend on certain files from some build, we could calculate a hash for only those to determine if we need to rebuild.
  (Probably means redoing how the store stores things to be based on files instead of directories, though, in order to not have terrible performance.)
