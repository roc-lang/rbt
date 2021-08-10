platform examples/hello-world
    requires {}{ main : { command: Str, arguments: List Str } }
    exposes []
    packages {}
    imports []
    provides [ mainForHost ]
    effects fx.Effect {}

mainForHost : { command: Str, arguments: List Str }
mainForHost = main
