Implement a FIFO queue of integers with fixed capacity. The first line holds the capacity C (1 <= C <= 100). Each following line is a command:

- `push x` : append x. Print `ok`, or `full` (and do nothing) if the queue already holds C items.
- `pop` : remove and print the oldest item, or print `empty` if there is none.
- `peek` : print the oldest item without removing it, or `empty`.
- `size` : print the number of items.
- `dump` : print the items from oldest to newest separated by spaces, or `<empty>` if there are none.

Print exactly one line per command.
