Standard input is a log; each well-formed line is `TIMESTAMP LEVEL COMPONENT message...` where TIMESTAMP has the exact form `YYYY-MM-DDTHH:MM:SS` with digits only, e.g. `2024-03-05T14:07:09` (so string order is time order), LEVEL is one of `DEBUG INFO WARN ERROR`, COMPONENT is a single word and the message is the rest of the line (may be empty). Any other line (including blank lines) is malformed.

Print:
1. `levels: DEBUG=<n> INFO=<n> WARN=<n> ERROR=<n>`
2. `top components:` followed by up to 3 lines `  <component> <count>` ordered by count descending then component ascending (counting all well-formed lines); nothing if there are none,
3. `first: <earliest timestamp>` and `last: <latest timestamp>` (compare as strings; print `first: -` / `last: -` if none),
4. `errors:` followed by the message of each ERROR line in input order, as `  [<component>] <message>`,
5. `malformed: <n>`.
