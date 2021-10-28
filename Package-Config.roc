platform roc-lang/rbt
    requires {}{ init : Rbt }
    exposes [ Rbt ]
    packages {}
    imports [ Rbt.{ Rbt } ]
    provides [ initForHost ]
    effects fx.Effect {}

# initForHost : Rbt
initForHost = init
