Each input line is one token (the whole line, including any spaces). Decide whether it is a valid signed 32-bit decimal integer: optional leading `-` or `+`, then one or more ASCII digits, nothing else (no spaces, no underscores, no decimals).

Print `ok <value>` for a valid in-range token (value printed in plain decimal, without `+` and without leading zeros, `-0` prints `0`). Otherwise print one of:
- `error empty` if the line is empty,
- `error invalid <token>` if it is not of the form above (including a lone sign),
- `error overflow <token>` if it is well formed but outside -2147483648..2147483647.
Finally print `summary ok=<a> invalid=<b> overflow=<c> empty=<d>` counting each outcome.
