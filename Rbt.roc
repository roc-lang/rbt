interface Rbt
    exposes [Rbt, init, Job, job, Command, exec, Tool, tool, systemTool, projectFiles, sourceFile]
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

FileMapping := Str

sourceFile : Str -> FileMapping
sourceFile = \name -> @FileMapping name

Input := [FromProjectSource (List FileMapping)]

# Add the given file to the job's workspace (the working directory where the
# command is called.)
projectFiles : List FileMapping -> Input
projectFiles = \mappings -> @Input (FromProjectSource mappings)

Job : [Job { command : Command, inputs : List Input, outputs : List Str }]

# TODO: these fields are all required until https://github.com/rtfeldman/roc/issues/1844 is fixed
# TODO: destructuring is broken, see https://github.com/rtfeldman/roc/issues/2512
job : { command : Command, inputs : List Input, outputs : List Str } -> Job
job = \config -> Job config

Rbt : { default : Job }

init : { default : Job } -> Rbt
init = \rbt -> rbt

tool : Job, Str -> Tool
tool = \_, _ ->
    # FromJob { name, job }
    @Tool (SystemTool { name: "TODO" })
