interface Rbt
  exposes [ Rbt, init, Job, job, Command, exec, Tool, tool, systemTool ]
  imports []

# TODO: make these all private
# TODO: these are all out of order due to https://github.com/rtfeldman/roc/issues/1642. Once that's fixed, they should rearrange into the order in `exposes`

Tool : [ @Tool { name: Str } ]

systemTool : Str -> Tool
systemTool = \name ->
    @Tool { name }

Command : { tool : Tool, args : List Str }

exec : Tool, List Str -> Command
exec = \execTool, args ->
    { tool: execTool, args }

Job : [ @Job { command : Command, inputs : List Job, inputFiles : List Str, outputs : List Str } ]

job : { command : Command, inputs ? List Job, inputFiles ? List Str, outputs : List Str } -> Job
job = \{ command, outputs, inputs ? [], inputFiles ? [] } ->
    @Job { command, inputs, inputFiles, outputs }

Rbt : [ @Rbt { default : Job } ]

init : { default : Job } -> Rbt
init = \rbt -> @Rbt rbt

tool : Job, Str -> Tool
tool = \_, outputName ->
    @Tool { name: "TODO" }
