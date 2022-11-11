app "build"
    packages { pf: "../../../Package-Config.roc" }
    imports [pf.Rbt.{ Rbt, systemTool, Job, job, exec, projectFiles, sourceFile, withFilename }]
    provides [init] to pf

init : Rbt
init =
    Rbt.init { default: hello }

hello : Job
hello =
    job {
        command: exec (systemTool "bash") [
            "-euo",
            "pipefail",
            "-c",
            """
            WHAT=$(cat what)
            WHO=$(cat who)
            
            printf '%s, %s!\n' "$WHAT" "$WHO" > out
            """,
        ],
        inputs: [
            projectFiles [
                sourceFile "subject"
                |> withFilename "who",
                sourceFile "greeting"
                |> withFilename "what",
            ],
        ],
        outputs: ["out"],
        env: Dict.empty,
    }
