platform "roc-lang/rbt"
    requires {} { init : Rbt }
    exposes [Rbt]
    packages {}
    imports []
    provides [initForHost]

initForHost : Rbt
initForHost = init

Tool : [SystemTool { name : Str }]

Command : [Command { tool : Tool, args : List Str }]

Job : [Job { command : Command, inputFiles : List Str, outputs : List Str }]

Rbt : [ Rbt { default : Job } ]
