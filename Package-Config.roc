platform roc-lang/rbt
    requires {}{ init : { default : Job } }
    exposes []
    packages {}
    imports [ Job.{ Job } ]
    provides [ initForHost ]
    effects fx.Effect {}

initForHost : { default : Job }
initForHost = init
