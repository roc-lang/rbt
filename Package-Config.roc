platform examples/hello-world
    requires {}{ main : { command: Str, arguments: List Str, inputs: List Str, outputs: List Str, workingDirectory: Str } }
    exposes []
    packages {}
    imports []
    provides [ mainForHost ]
    effects fx.Effect {}

mainForHost : { command: Str, arguments: List Str, inputs: List Str, outputs: List Str, workingDirectory: Str }
mainForHost = main