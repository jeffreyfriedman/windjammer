The input is a configuration in `key=value` lines (split at the first `=`; no whitespace trimming is needed - inputs have none). Blank lines are ignored. If a key repeats, the last value wins. Unknown keys are errors.

Schema:
- `host`: required, must be non-empty.
- `port`: required, integer 1..65535.
- `debug`: optional, `true` or `false`, default `false`.
- `workers`: optional, integer 1..64, default 4.

Collect ALL problems and print them (not just the first), in this fixed order: for each schema field in the order host, port, debug, workers: `error: <field>: missing` (required but absent), `error: <field>: empty` (host present but empty), or `error: <field>: invalid value <value>` (present but not parseable/in range); then `error: unknown key <key>` for each unknown key in order of first appearance. A line without `=` is reported as `error: malformed line <n>` (1-based line number among all lines including blank ones) and appears before everything else, in line order.

If there are no problems print `ok host=<host> port=<port> debug=<debug> workers=<workers>`. Integers must be plain decimal digits (no sign, no spaces); leading zeros are allowed and the `ok` line prints the integer value without them (`port=05` prints `port=5`).
