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
            "-euo",
            "pipefail",
            "-c",
            """
            H=$(cat h)
            E=$(cat e)
            L=$(cat l)
            O=$(cat o)
            
            printf '%s%s%s%s%s' $H $E $L $L $O > greeting
            """,
        ],
        inputs: [
            fromJob h [sourceFile "h"],
            fromJob e [sourceFile "e"],
            fromJob l [sourceFile "l"],
            fromJob o [sourceFile "o"],
        ],
        outputs: ["greeting"],
        env: Dict.empty,
    }

subject : Job
subject =
    job {
        command: exec (systemTool "bash") [
            "-euo",
            "pipefail",
            "-c",
            """
            W=$(cat w)
            O=$(cat o)
            R=$(cat r)
            L=$(cat l)
            D=$(cat d)
            
            printf '%s%s%s%s%s' $W $O $R $L $D > subject
            """,
        ],
        inputs: [
            fromJob w [sourceFile "w"],
            fromJob o [sourceFile "o"],
            fromJob r [sourceFile "r"],
            fromJob l [sourceFile "l"],
            fromJob d [sourceFile "d"],
        ],
        outputs: ["subject"],
        env: Dict.empty,
    }

letter : Str -> Job
letter = \whichLetter ->
    job {
        command: exec (systemTool "bash") [
            "-c",
            "printf \(whichLetter) > \(whichLetter)",
        ],
        inputs: [],
        outputs: [whichLetter],
        env: Dict.empty,
    }

d = letter "d"
e = letter "e"
h = letter "h"
l = letter "l"
o = letter "o"
r = letter "r"
w = letter "w"
