The first line holds `W H G`: grid width, height (1..30 each) and the number of generations G (0..200). The next H lines each hold W characters, `#` (alive) or `.` (dead).

Simulate G generations of Conway's Game of Life on a TORUS (edges wrap around in both directions). A live cell with 2 or 3 live neighbors (out of 8) stays alive, a dead cell with exactly 3 live neighbors becomes alive, all others die/stay dead. On a very small grid a cell may count the same neighbor cell more than once because of wrapping; count by neighbor POSITION (each of the 8 offsets counts, even if it wraps to the same cell).

Print the final grid (H lines) and then `alive: <count>`.
