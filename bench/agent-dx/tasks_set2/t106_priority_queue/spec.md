Commands, one per line:

- `add NAME PRIORITY` : enqueue a job (PRIORITY is an integer, possibly negative)
- `pop` : remove and print the job with the highest priority; among equal priorities the job added first. Prints `NAME` or `empty`.
- `peek` : like `pop` but does not remove the job
- `size` : print the number of queued jobs

Unknown commands or malformed lines print `error`. After all commands print `left:` followed by the remaining jobs in pop order, one `NAME PRIORITY` per line.
