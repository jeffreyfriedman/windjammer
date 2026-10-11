Each input line is a string of 0 to 200 lower-case letters. For every line print `<len> <sub> <count>`:
- `sub` is the longest palindromic substring (the leftmost one if several have the maximal length), `len` its length;
- `count` is the number of palindromic substrings counted by position (every (start, end) pair whose substring is a palindrome, so `aaa` has 6).
For an empty line print `0 - 0`.
