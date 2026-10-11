The first line is `enc N` or `dec N` with an integer N (it may be negative or larger than 26). The remaining lines are text.

Shift every ASCII letter by N positions in the alphabet (`enc`) or by -N (`dec`), wrapping around and preserving case. All other characters are left unchanged. Print the transformed text with line structure preserved.
