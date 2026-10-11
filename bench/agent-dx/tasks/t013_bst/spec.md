Implement a binary search tree of distinct integers (no balancing required). Commands, one per line:

- `insert x` : insert x; print `ok`, or `dup` if already present.
- `contains x` : print `yes` or `no`.
- `delete x` : remove x if present (print `ok`) else print `missing`. When deleting a node with two children replace it with its in-order successor.
- `inorder` : print all keys in ascending order separated by spaces, or `<empty>`.
- `height` : number of nodes on the longest root-to-leaf path (0 for an empty tree).
- `min` / `max` : print the smallest / largest key, or `empty`.
