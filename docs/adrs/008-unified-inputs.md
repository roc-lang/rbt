# ADR 008: Unified Inputs

We want to be able to specify inputs of various kinds to a `Job`.
The API we've kicked around for this has looked like a lot of different fields on `Job`:

- `inputs` for jobs that the `Job` depends on
- `inputFiles` for files from the project
- `inputPatterns` (speculatively) for discoverable files
- `dynamicInputs` (speculatively) for [dynamic dependencies](./005-dynamic-dependencies.md)

That's a lot of fields for something that essentially works out to "I need these files. Can I please have them?"
It also does not allow us to check that the files in build steps actually exist (same problem as splat imports: say you produce some file in an `inputs` job.
Your build script depends on it, but then it goes away.
We have no idea where that file was supposed to have come from, so we can't help!)

Instead, this ADR proposes that we unify these fields into `inputs : List Input` (really `Set Input` as soon as possible.)

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
(As a matter of fact, matchers might take over `FromSource`'s job!)

# Benefits

This API:

- Lets us specify exactly how jobs want the filesystem to be set up before they run
- Lets us warn about conflicts between different files.
- Lets us see exactly where we're trying to source files from.
  This means we can see when a file would not exist and warn about that.
  (`allOutputsOf` defeats this a bit since it automatically depends on whatever files are available, but overall I think it's a win.)
- Opens up new optimization opportunities for caching: if we know that we only depend on certain files from some build, we could calculate a sub-hash for those.
  (Probably means redoing how the store stores things to be based on files instead of directories, though, in order to not have abysmal performance.)
