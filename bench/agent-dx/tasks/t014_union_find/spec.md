The first line holds N (1 <= N <= 1000): elements are numbered 0..N-1 and start in singleton sets. Each following line is a command:

- `union a b` : merge the sets of a and b. Print `merged` if they were different sets, `same` otherwise.
- `find a b` : print `yes` if a and b are in the same set else `no`.
- `size a` : print the size of the set containing a.

After the last command print `components <k>` where k is the number of distinct sets.
