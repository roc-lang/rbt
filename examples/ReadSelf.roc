#!/usr/bin/env roc run

app "rbt"
    packages { base: ".." }
    imports []
    provides [ main ] to base

main =
    { 
        command: "cp",
        arguments: [ "examples/ReadSelf.roc", "examples/ReadSelf.roc.copy" ],
        inputs: [ "examples/ReadSelf.roc" ],
        outputs: [ "examples/ReadSelf.roc.copy" ],
    }
