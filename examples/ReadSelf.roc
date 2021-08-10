#!/usr/bin/env roc run

app "rbt"
    packages { base: ".." }
    imports []
    provides [ main ] to base

main =
    { 
        command: "cat",
        arguments: [ "examples/ReadSelf.roc" ],
    }
