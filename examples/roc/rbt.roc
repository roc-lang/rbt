app "build"
    packages { base: "rbt" }
    imports [ base.Job.{ Job, cwd }, base.Cmd.{ cmd }, base.Exec ]
    provides [ init ] to base

## rbt uses **commands** and **jobs**.
##
## * A **command** is a CLI command. When you run `rbt test`, it will execute the `test` *command*.
## * A **job** is a description of how some work gets done. Each command consists of 1 or more *jobs*. Jobs can be run sequentially, in parallel, or in some combination of those.
##
## You can always use the subcommand `watch` to watch a command instead of
## running it once - e.g. `rbt watch test` or `rbt watch` to watch the default
## command.
init : Rbt.Cli
init =
    Rbt.Cli.init
        [
            # "" is the default cmd - gets run when you run `rbt` alone
            cmd "" (Job.seq [ fmt, build, clippy ]), # Job.seq means run sequentially
            cmd "ci" (Job.seq [ fmt, build, test, clippy ]),
            cmd "test" (Job.par [ zigTest, rustTest ]), # Job.par means run in parallel
            cmd "test-zig" zigTest,
            cmd "test-rust" rustTest,
        ]
        {
            alwaysIgnore:
                [
                    ".git",
                    ".github",
                    ".gitignore",
                    ".llvmenv",
                    "compiler/builtins/zig-cache/**/*",
                ]
        }

zigTest : Job
zigTest =
    Job.single
        {
            name: "zig test",
            watches: [ "compiler/builtins/bitcode/**/*.zig" ],
            touches: [ "compiler/builtins/bitcode/zig-cache/" ],
            whenFilesChanged:
                exec "zig build test"
                    |> Exec.cwd "bitcode"
        }

rustTest : Job
rustTest =
    Job.single
        {
            name: "cargo test --release",
            roots: [ "Cargo.toml" ],
            # For each file in `roots`, run this on the file to determine
            # what files it depends on (e.g. by parsing it).
            # Whenever any of those files changes on disk, this job must be
            # re-run on that particular file!
            deps: exec "find_test_deps_from_cargo_toml.pl",
            touches: [ "target/" ],
            forEachChangedFile: exec "cargo test --release --test",
            onSuccess: exec "sccache --show-stats",
        }

clippy : Job
clippy =
    Job.single
        {
            name: "cargo clippy",
            roots: [ "Cargo.toml" ],
            deps: exec "find_clippy_deps_from_cargo_toml.pl",
            touches: [ "target/" ],
            whenFilesChange: exec "cargo clippy -- -D warnings",
        }

fmt : Job
fmt =
    Job.par
        [
            # find src/*.zig -type f -print0 | xargs -n 1 -0 zig fmt --check
            Job.single
                {
                    name: "zig fmt --check",
                    watches: [ "compiler/builtins/bitcode/**/*.zig" ],
                    forEachChangedFile: exec "zig fmt --check",
                },
            Job.single
                {
                    name: "cargo fmt --check",
                    watches: [ "./**/*.rs" ],
                    whenFilesChange:
                        exec "cargo fmt --all -- --check"
                            |> Exec.env {{ "RUST_BACKTRACE" => "1" }},
                }
        ]
