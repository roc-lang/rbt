#!/usr/bin/env roc run

app "rbt"
    packages { base: ".." }
    imports []
    provides [ main ] to base

greeting =
    hi = "Hello"
    name = "World"

    "\(hi), \(name)!\n"

main = [ greeting, "foo.elm", "Bar.elm" ]
