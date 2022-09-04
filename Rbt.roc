interface Rbt
    exposes [Rbt, init, Job, job, Command, exec, Tool, tool, systemTool]
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
Tool : [SystemTool { name : Str }]

systemTool : Str -> Tool
systemTool = \name ->
    SystemTool { name }

Command : [Command { tool : Tool, args : List Str, env : Dict Str Str }]

exec : Tool, List Str, Dict Str Str -> Command
exec = \execTool, args, env ->
    Command { tool: execTool, args, env }

Job : [Job { command : Command, inputFiles : List Str, outputs : List Str }]

# TODO: these fields are all required until https://github.com/rtfeldman/roc/issues/1844 is fixed
# TODO: destructuring is broken, see https://github.com/rtfeldman/roc/issues/2512
job : { command : Command, inputFiles : List Str, outputs : List Str } -> Job
job = \stuff ->
    Job { command: stuff.command, inputFiles: stuff.inputFiles, outputs: stuff.outputs }

Rbt : { default : Job }

init : { default : Job } -> Rbt
init = \rbt -> rbt

tool : Job, Str -> Tool
tool = \_, _ ->
    # FromJob { name, job }
    SystemTool { name: "TODO" }
