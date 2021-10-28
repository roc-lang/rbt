app "build"
    packages { pf: "../.." }
    imports [ pf.Rbt.{ Rbt, Tool, tool, systemTool, Job, job, exec } ]
    provides [ init ] to pf


# todo: bikeshed "init" name more
init : Rbt
init =
    Rbt.init { default: bundle }


# note: these rules could be much more compact but we're spelling them out
# explicitly for ease of understanding. Files using rbt do not have to be
# so verbose!
nixShell : Tool
nixShell =
    systemTool "nix-shell"


wat2wasmBinary : Job
wat2wasmBinary =
    job
        {
            command: exec nixShell [ "-p", "wabt", "--run", "ln -s $(which wat2wasm) wat2wasm" ],
            outputs: [ "wat2wasm" ]
        }


wat2wasm : Tool
wat2wasm =
    tool wat2wasmBinary "wat2wasm"


esbuildBinary : Job
esbuildBinary =
    job
        {
            command: exec nixShell [ "-p", "esbuild", "--run", "ln -s $(which esbuild) esbuild" ],
            outputs: [ "esbuild" ]
        }


esbuild : tool
esbuild =
    tool esbuildBinary "esbuild"


#######################################
# Done with tools, now for the build! #
#######################################


addWasm : Job
addWasm =
    job
        {
            command: exec wat2wasm [ "hello.wat" ],
            inputFiles: [ "hello.wat" ],
            outputs: [ "hello.wasm" ],
        }


bundle : Job
bundle =
    job
        {
            command:
                exec esbuild
                    [
                        "--platform=node",
                        "--bundle",
                        "--loader:.wasm=binary",
                        "--minify",
                        "--sourcemap",
                        "--outfile=index.min.js",
                        "index.js",
                    ],
            inputs: [ addWasm ],
            inputFiles: [ "index.js" ],
            outputs: [ "index.min.js", "index.min.js.map" ],
        }
