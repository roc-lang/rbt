app "build"
    packages { pf: "../../main.roc" }
    imports [pf.Rbt.{ Rbt, systemTool, Job, job, exec }]
    provides [init] to pf

init : Rbt
init =
    Rbt.init { default: hello }

hello : Job
hello =
    job {
        command: exec (systemTool "bash") ["-c", "echo 'Hello, World!' > out"],
        inputFiles: [],
        outputs: ["out"],
    }
