# Install

Windjammer ships as one program, `wj`. It contains the compiler, the interpreter, the formatter, the language server
and the project tools. Compiled programs go through Rust, so you also want a Rust toolchain installed
([rustup.rs](https://rustup.rs) takes a minute).

## Build from source

<!-- ignore: builds the compiler itself; CI builds wj before it runs the verified commands in this book -->
```sh,ignore
git clone https://github.com/jeffreyfriedman/windjammer.git
cd windjammer
cargo build --release
export PATH="$PWD/target/release:$PATH"
```

Add the `export` line to your shell profile to keep `wj` on your path. The repository also has an `install.sh` that
does the same steps and puts `wj` somewhere permanent.

## Check the installation

```sh,verify
$ wj --version
wj
```

The version line carries the commit and build date, so a bug report can say exactly which `wj` it came from.

`wj doctor` looks for stale or duplicate copies of `wj` on your path and prints the command that fixes each one. Run it
whenever something behaves oddly after an update; it finishes with `no problems found` when all is well.

## Editors

The language server lives in the repository under `crates/windjammer-lsp`, and a VS Code extension under
`vscode-extension/`. Both give you diagnostics, hover, completion and formatting while you type.

## Next

Create your first project in [Your first project](first-project.md).
