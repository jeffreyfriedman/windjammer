Implement a least-recently-used cache from integer keys to integer values. The first line holds the capacity C (1 <= C <= 50). Each following line is a command:

- `put k v` : insert or update key k. The key becomes the most recently used. If the cache then holds more than C entries, evict the least recently used one. `put` prints nothing, except when an eviction happened: then print `evict <key>` with the evicted key.
- `get k` : print the value, or `-1` if absent. A successful get makes the key most recently used.
- `keys` : print the keys from most to least recently used separated by spaces, or `<empty>`.
