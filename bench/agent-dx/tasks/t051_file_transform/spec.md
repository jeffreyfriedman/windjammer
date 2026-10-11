Arguments: `INPUT OUTPUT MODE`. Read the text file INPUT (ASCII, lines separated by `\n`; a trailing newline does not create an extra empty line), transform its lines according to MODE, write the result to the file OUTPUT (create or overwrite; every line ends with `\n`), then print `wrote <n> lines to <OUTPUT>` followed by the contents of OUTPUT read back from disk.

MODE values: `upper` (upper-case every line), `reverse` (reverse the ORDER of lines), `sort` (sort lines ascending bytewise), `dedupe` (remove repeated lines keeping the first occurrence), `trim` (strip leading and trailing spaces of every line), `number` (prefix each line with its 1-based number and a colon, e.g. `3:text`).

If INPUT is missing print `error: cannot read <INPUT>` and exit with status 1 (no OUTPUT is written). If MODE is unknown print `error: unknown mode <MODE>` and exit with status 2. Wrong argument count prints `usage: transform INPUT OUTPUT MODE` and exits 2.
