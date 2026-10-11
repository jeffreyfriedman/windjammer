Each input line is a string that may contain the brackets `()[]{}` and any other characters (which are ignored). Check that brackets are balanced and properly nested.

Print `ok` if the line is balanced. Otherwise print `error <i>` where i is the 1-based index (counting every character of the line) of:
- the first closing bracket that has no matching opener or does not match the most recent unclosed opener; or, if no such closer exists,
- the most recent unclosed opener remaining at the end of the line.
Blank lines are `ok`.
