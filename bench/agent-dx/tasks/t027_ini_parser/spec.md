The input has two parts separated by a line that is exactly `---`. Before it is an INI document; after it are query lines.

INI rules: a line `[name]` starts section `name`; `key = value` sets a key in the current section (split at the first `=`, trim spaces around key and value); lines starting with `;` or `#` (after trimming) and blank lines are ignored. Keys before any section belong to the global section (empty name). Later assignments to the same key override earlier ones. Lines that are none of the above are ignored.

Each query is `section.key` (split at the FIRST `.`), or just `key` (no dot) for the global section. Print the value, or `<none>` if the section/key is not defined.
