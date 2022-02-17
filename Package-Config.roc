platform "roc-lang/rbt"
    requires {}{ init : Rbt }
    exposes [ Rbt ]
    packages {}
    imports []
    provides [ initForHost ]

# initForHost : Rbt
initForHost = init
