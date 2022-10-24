app "build"
    packages { pf: "../../../Package-Config.roc" }
    imports [pf.Rbt.{ Rbt, systemTool, Job, job, exec }]
    provides [init] to pf

init : Rbt
init =
    Rbt.init { default: hello }

hello : Job
hello =
    job {
        command: exec (systemTool "bash") [
            "-c",
            "env > out",
        ],
        inputs: [],
        outputs: ["out"],
        env: Dict.empty
    }
