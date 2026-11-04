Usage: the program receives file names as arguments. For each readable file (in argument order) count its lines (number of `\n` characters) and whitespace-separated words, and print `NAME: LINES lines, WORDS words`. A file that cannot be read prints `NAME: cannot read` instead.

After processing all arguments, write a file named `report.txt` in the current directory containing one line per argument in the same order: `NAME LINES WORDS` for readable files and `NAME -` for unreadable ones (each line ends with `\n`). Then read `report.txt` back and print `report:` on its own line, followed by the file's contents exactly as stored.

If no arguments are given print `usage: tally FILE...` and exit with status 2. If at least one file was unreadable exit with status 1 (after printing everything); otherwise exit 0.
