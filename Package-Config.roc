platform "roc-lang/rbt"
    requires {}{ init : Rbt }
    exposes [ Rbt ]
    packages {}
    imports []
    provides [ initForHost ]

initForHost : Rbt
initForHost = init

Tool : [ SystemTool { name: Str } ]

Command : [ Command { tool : Tool } ]

Job : [ Job { command : Command, inputFiles : List Str } ]

Rbt : { default: Job }
