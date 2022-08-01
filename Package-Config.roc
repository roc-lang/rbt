platform "roc-lang/rbt"
    requires {} { init : Rbt }
    exposes [Rbt]
    packages {}
    imports []
    provides [initForHost]

initForHost : Rbt
initForHost = init

# TODO: once `roc glue` knows how to resolve them, these should move back
# into Rbt.roc so we can stop copying the definitions over every time we make
# a change!

Tool : [SystemTool { name : Str }]

Command : [Command { tool : Tool, args : List Str }]

Job : [Job { command : Command, inputFiles : List Str, outputs : List Str }]

Rbt : [ Rbt { default : Job } ]
