interface Rbt
  exposes [ Rbt, init, Job, job, Command, exec, Tool, tool, systemTool ]
  imports []

Rbt : { default : Job }

init : { default : Job } -> Rbt
init = \rbt -> rbt

#####

Job : [ @Job { command : Command, inputs : List Job, inputFiles : List Str, outputs : List Str } ]

job : { command : Command, inputs ? List Job, inputFiles ? List Str, outputs : List Str } -> Job
job = \{ command, outputs, inputs ? [], inputFiles ? [] } ->
    @Job { command, inputs, inputFiles, outputs }

#####

Command : [ @Command { tool : Tool, args : List Str } ]

exec : Tool, List Str -> Command
exec = \tool, args ->
    @Command { tool, args }

#####

Tool : [ @Tool Str ]

systemTool : Str -> Tool
systemTool = \name ->
    @Tool name

tool : Job, Str -> Tool
tool = \job, outputName ->
    @Tool "TODO"
