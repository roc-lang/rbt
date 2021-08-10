#!/usr/bin/env roc run

app "rbt"
    packages { base: ".." }
    imports []
    provides [ main ] to base

main = [ "examples/ReadSelf.roc" ]
