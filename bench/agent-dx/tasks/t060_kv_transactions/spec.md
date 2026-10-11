Implement an in-memory key-value store (string keys without spaces, integer values) with nested transactions. Commands, one per line:

- `set K V` : set the key (inside a transaction the change is only visible until rolled back). Print nothing.
- `get K` : print the value or `NULL`.
- `delete K` : remove the key. Print nothing.
- `count V` : print how many keys currently hold the value V.
- `begin` : open a new (possibly nested) transaction. Print nothing.
- `rollback` : undo everything done since the most recent `begin` and close it; print `NO TRANSACTION` if none is open.
- `commit` : make ALL open transactions permanent (close them all); print `NO TRANSACTION` if none is open.
