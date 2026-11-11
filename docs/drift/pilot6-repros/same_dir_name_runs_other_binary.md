# `wj run --compiled` (and `--verify`) runs the other program when two project directories share a name

Two projects whose directories have the same final name (`app` below) share one cargo package name, and so one binary in the shared target directory.
Running project X, then project Y, then project X again (unchanged) prints Y's output for the second run of X.

```sh
mkdir -p x/app y/app
cp same_dir_name_x/main.wj x/app/main.wj      # prints "program X"
cp same_dir_name_y/main.wj y/app/main.wj      # prints "program Y"
for d in x y x y; do echo "$d: $(cd $d/app && wj run --compiled main.wj </dev/null 2>/dev/null | tail -1)"; done
```

Observed (pilot 6 compiler, 2026-11-11):

```
x: program X
y: program Y
x: program Y      <- X's own build is up to date, the binary on disk is Y's
y: program Y
```

With directories named differently (`a/`, `b/`) the output is correct, so the cause is the package name taken from the directory name.
In the pilot every agent workspace is called `ws`, so two trials running at once (parallelism 2) printed each other's output through `wj run --verify`
("stdout differs ... compiled: invalid input"), and the agents chased a difference that was not in their program. Tool-side fix to evaluate:
a package name derived from the stable per-source work directory hash, or running the binary the build reports instead of `cargo run`.
