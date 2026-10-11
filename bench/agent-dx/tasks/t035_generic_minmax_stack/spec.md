Implement ONE generic stack type that works for both integers and strings and can report its minimum and maximum in O(1). The first line is `int` or `str` and selects the element type. Each following line is a command:

- `push X` : push X (for `int` parse a signed 64-bit integer, for `str` the rest of the line after `push ` is the string, which may contain spaces and may be empty). Print nothing.
- `pop` : remove and print the top, or `empty`.
- `min` / `max` : print the smallest / largest element currently in the stack, or `empty`. Strings compare bytewise (lexicographically).
- `size` : print the element count.
