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

The API looks something like this:

```coffeescript
Input := # private, hidden

FileMapping := # private, hidden

sourceFile : Str -> FileMapping

withFilename : FileMapping, Str -> FileMapping

fromProject : List FileMapping -> Input

fromJob : Job, List FileMapping -> Input
```

Usage might look like this:

```coffeescript
completedGreeting : Job
completedGreeting =
    job {
        command: exec (systemTool "bash") [
            "-c",
            """printf '%s, %s!\n' "$(cat greeting.txt)" "$(cat subject.txt)" > completedGreeting.txt""",
        ],
        inputs: [
            fromProject [sourceFile "subject.txt"],
            fromJob greeting [sourceFile "englishGreeting.txt" |> withFilename "greeting.txt"],
        ],
        outputs: ["completedGreeting.txt"]
    }

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
- Opens up new optimization opportunities for caching: if we know that we only depend on certain files from some build, we could calculate a hash for only those to determine if we need to rebuild.
  (Probably means redoing how the store stores things to be based on files instead of directories, though, in order to not have terrible performance.)
- Can be extended with things like pattern inputs or functions to modify all the input files at once (e.g. `inWorkspaceDirectory : Input, Str -> Input`.)
