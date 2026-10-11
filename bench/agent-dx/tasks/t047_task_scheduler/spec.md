Simulate (no real threads needed) a scheduler with W identical workers. The first line holds `W N`. The next N lines describe tasks: `id duration dep1 dep2 ...` where id is a unique integer, duration a positive integer, and the deps are ids of tasks that must FINISH before this task may start (possibly none). The dependency graph is acyclic and every dep id exists.

Time starts at 0. Whenever a worker is free and a task is ready (all deps finished at or before the current time), the ready task with the SMALLEST id is started on a free worker. Several tasks may start at the same instant: assign them in increasing id order. A task finishing at time t frees its worker at time t, and tasks that depend on it can start at time t. Time advances to the next finishing time when nothing more can start.

Print `<id> <start> <end>` for each task sorted by id ascending, then `makespan <end of the last finishing task>` (0 if N = 0).
