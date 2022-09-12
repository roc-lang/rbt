app "build"
    packages { pf: "../../../Package-Config.roc" }
    imports [pf.Rbt.{ Rbt, systemTool, Job, job, exec, file }]
    provides [init] to pf

init : Rbt
init =
    Rbt.init { default: hello }

hello : Job
hello =
    job {
        command: exec (systemTool "bash") [
            "-c",
            "printf '%s, %s!\n' \"$(cat greeting)\" \"$(cat subject)\" > out",
        ],
        inputs: [
            file "greeting",
            file "subject",
        ],
        outputs: ["out"],
    }

