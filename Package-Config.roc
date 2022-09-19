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
SystemToolPayload : { name : Str }
Tool : [SystemTool SystemToolPayload]

CommandPayload : { tool : Tool, args : List Str }
Command : [Command CommandPayload]

FileMapping : Str
Input : [FromProjectSource (List FileMapping)]

Job : [Job { command : Command, inputs : List Input, outputs : List Str }]

Rbt : { default : Job }
