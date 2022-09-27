app "build"
    packages { pf: "../../../Package-Config.roc" }
    imports [pf.Rbt.{ Rbt, systemTool, Job, job, exec, projectFiles, sourceFile, fromJob }]
    provides [init] to pf

init : Rbt
init =
    Rbt.init { default: helloWorld }

helloWorld : Job
helloWorld =
    job {
        command: exec (systemTool "bash") [
            "-euo",
            "pipefail",
            "-c",
            """
            GREETING="$(cat greeting)"
            SUBJECT="$(cat subject)"
            printf '%s, %s!\n' "$GREETING" "$SUBJECT" > out
            """,
        ],
        inputs: [
            fromJob greeting [sourceFile "greeting"],
            fromJob subject [sourceFile "subject"],
        ],
        outputs: ["out"],
        env: Dict.empty,
    }

greeting : Job
greeting =
    job {
        command: exec (systemTool "bash") [
            "-c",
            "printf Hello > greeting",
        ],
        inputs: [],
        outputs: ["greeting"],
        env: Dict.empty,
    }

subject : Job
subject =
    job {
        command: exec (systemTool "bash") [
            "-c",
            "printf World > subject",
        ],
        inputs: [],
        outputs: ["subject"],
        env: Dict.empty,
    }
