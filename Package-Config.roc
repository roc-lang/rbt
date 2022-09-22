platform "roc-lang/rbt"
    requires {} { init : Rbt.Rbt }
    exposes [Rbt]
    packages {}
    imports [Rbt]
    provides [initForHost]

initForHost : Rbt.Rbt
initForHost = init
