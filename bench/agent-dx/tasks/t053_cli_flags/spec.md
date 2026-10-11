Write the argument parser of a tool. Recognized arguments:

- `--name=VALUE` or `--name VALUE` : string option (later occurrences override earlier ones; default `anonymous`)
- `--count=N` or `--count N` : integer option, default 1; must be a non-negative decimal integer
- `-v` : verbosity, may repeat (`-v -v`) and may be clustered (`-vvv` counts 3)
- `--dry-run` : boolean flag
- `--` : everything after it is positional, even if it starts with `-`
- any other argument not starting with `-` is positional; a lone `-` is positional.
- any other argument starting with `-` (before `--`) is an error.

Print exactly these five lines in this order (the last line has nothing after `positional:` when there are no positionals):

```
name: <name>
count: <count>
verbosity: <v>
dry-run: <true|false>
positional: <positional arguments joined by single spaces>
```

Errors print a single line instead of the five lines, and the program exits with status 2. Process arguments left to right; the first error wins:
- `error: unknown option <arg>` for an unrecognized argument starting with `-` (before `--`),
- `error: missing value for --name` / `error: missing value for --count` when the space-separated form is the last argument,
- `error: invalid count <value>` when the count value is not a plain non-negative decimal integer (leading zeros are fine).
For the space-separated forms the NEXT argument is always consumed as the value, even if it starts with `-`. `--name=` gives an empty name. `--dry-run=1` is an unknown option. A cluster such as `-vx` is an unknown option (`error: unknown option -vx`).
