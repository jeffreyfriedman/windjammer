Each input line is `E text` or `D text`, where text is a (possibly empty) string of lower-case letters for `E`, or an encoded string for `D`.

- `E`: run-length encode: every maximal run of the same letter becomes `<count><letter>` (so `aaabcc` -> `3a1b2c`). Counts can exceed 9.
- `D`: decode the same format (counts may have several digits).

Print the result for each line; an empty result is an empty line.
