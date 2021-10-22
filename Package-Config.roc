platform roc-lang/rbt
    requires {}{ init : Rbt }
    exposes [ Rbt, Tool, Job, Command ]
    packages {}
    imports [ Job.{ Job }, Rbt.{ Rbt } ]
    provides [ initForHost ]
    effects fx.Effect {}

initForHost : Rbt
initForHost = init
