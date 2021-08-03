platform examples/hello-world
    requires {}{ main : List Str }
    exposes []
    packages {}
    imports []
    provides [ mainForHost ]
    effects fx.Effect {}

mainForHost : List Str
mainForHost = main
