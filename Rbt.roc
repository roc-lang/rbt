interface Rbt
    exposes [Rbt, init, Job, job, Command, exec, Tool, tool, systemTool, projectFiles, fromJob, Input, sourceFile, withFilename]
    imports []

# TODO: these are all out of order due to https://github.com/rtfeldman/roc/issues/1642. Once that's fixed, they should rearrange into the order in `exposes`
# TODO: how are we gonna get tools from Jobs? Maybe Tool, Command, and Job
# need to live in a single union and have private aliases outwards?  I'd like
# to have this look like:
#
#     Tool : [ Tool { name : Str, fromJob: Maybe Job } ]
#
# Or maybe:
#
#     Tool : [ SystemTool { name : Str }, FromJob { name : Str, job : Job } ]
#
SystemToolPayload : { name : Str }
Tool := [SystemTool SystemToolPayload]

systemTool : Str -> Tool
systemTool = \name ->
    @Tool (SystemTool { name })

Command := { tool : Tool, args : List Str }

exec : Tool, List Str -> Command
exec = \execTool, args ->
    @Command { tool: execTool, args }

FileMapping := { source : Str, dest : Str }

sourceFile : Str -> FileMapping
sourceFile = \name -> @FileMapping { source: name, dest: name }

withFilename : FileMapping, Str -> FileMapping
withFilename = \@FileMapping { source }, dest -> @FileMapping { source, dest }

Input := [
    FromProjectSource (List FileMapping),
    FromJob Job (List FileMapping),
]

# Add the given file to the job's workspace (the working directory where the
# command is called.)
projectFiles : List FileMapping -> Input
projectFiles = \mappings -> @Input (FromProjectSource mappings)

# Add files from the given job to the current job's workspace.
fromJob : Job, List FileMapping -> Input
fromJob = \otherJob, mappings -> @Input (FromJob otherJob mappings)

Job := [
    Job
        {
            command : Command,
            # eventually we want this to be `List Input` but there's a bug.
            # see https://github.com/roc-lang/roc/issues/4077
            inputs : List Input,
            outputs : List Str,
            env : Dict Str Str,
        },
]

# TODO: these fields are all required until https://github.com/rtfeldman/roc/issues/1844 is fixed
# TODO: destructuring is broken, see https://github.com/rtfeldman/roc/issues/2512
job : { command : Command, inputs : List Input, outputs : List Str, env : Dict Str Str } -> Job
job = \{ command, inputs, outputs, env } ->
    @Job (Job { command, inputs, outputs, env })

Rbt := { default : Job }

init : { default : Job } -> Rbt
init = \rbt -> @Rbt rbt

tool : Job, Str -> Tool
tool = \_, _ ->
    # FromJob { name, job }
    @Tool (SystemTool { name: "TODO" })
