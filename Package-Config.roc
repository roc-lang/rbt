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
SystemTool : { name : Str }

Tool : [SystemTool SystemTool]

CommandGuts : { tool : Tool, args : List Str }

Command : [Command CommandGuts]

JobGuts : { command : Command, inputFiles : List Str, outputs : List Str }

Job : [Job JobGuts]

RbtGuts : { default : Job }

Rbt : [Rbt RbtGuts]
