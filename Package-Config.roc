platform "roc-lang/rbt"
    requires {}{ init : Rbt }
    exposes [ Rbt ]
    packages {}
    imports [ pf.Rbt.{ Rbt } ]
    provides [ initForHost ]

# initForHost : Rbt
initForHost = init
