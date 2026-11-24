Simulate a tiny text-adventure world. The input has two parts: world lines, then a line holding exactly `---`, then command lines. Without a `---` line the whole input is world lines and there are no commands. Empty lines are ignored in both parts (but world lines are still numbered by their position in the input, starting at 1). All fields are separated by single spaces. Names are non-empty tokens without spaces. Numbers (WEIGHT, CAP, LIMIT) are digits only with at most 6 digits (leading zeros allowed).

**Line splitting (exact, no trimming).** Fields are obtained by splitting the line on single space characters, exactly. A leading, trailing or doubled space therefore produces an empty field, and the line is handled like any other line with a wrong number of fields or a malformed token (for example ` open a 1` is not `open a 1`). Only a line of length zero is an empty line: a line that holds only spaces is not.

World lines, processed in order. A line that is malformed, or refers to something not yet declared, or declares a duplicate, prints `error: bad world line N` (N is its line number) and has no other effect:

- `room NAME` declares a room.
- `exit FROM DIR TO [KEY]` adds a one-way exit from room FROM in direction DIR (one of `n s e w u d`) to room TO. A second exit from the same room in the same direction is bad. With KEY the exit is a locked door until unlocked; KEY is an item name that need not exist.
- `thing NAME WEIGHT LOCATION` declares an item. NAME must be new among all items. LOCATION is a declared room name, or `in:BOX` where BOX is an already declared box.
- `box NAME WEIGHT CAP LOCATION [open]` declares a container item (same rules, CAP is its capacity); it starts closed unless the word `open` ends the line. Declarations do not check capacity.
- `player ROOM LIMIT` places the player in a declared room; LIMIT is the most weight the player can carry. Only the first valid `player` line counts; later ones are bad.

If no valid `player` line exists, print `error: no player` after the world errors and stop (exit status 1, no commands run). The player starts carrying nothing.

Weights. The total weight of an item is its own weight plus the total weights of everything inside it, recursively. The carried weight is the sum of the total weights of the items the player carries directly.

Visible items: the items directly in the player's room or directly carried. Items inside a box are not visible, open or not. All command names refer to items by name.

Commands (one per line; any line that is not one of these forms prints `error: bad command`). Checks are made in the order listed, and the first failing check prints its error and changes nothing:

- `look` prints three lines: `== ROOM ==`; `exits: ` followed by the exit directions available from the room in the order n s e w u d separated by spaces, each followed by `(locked)` while locked, or `exits: none`; `items: ` followed by the items directly in the room sorted by name and separated by `, `, or `items: nothing`. A thing renders as its name. A closed box renders as `NAME{closed}`, an open box as `NAME{` + its direct contents rendered the same way, sorted by name, separated by `, ` + `}` (so an empty open box is `NAME{}`).
- `inv` prints `carrying: ` followed by the directly carried items rendered like the room items (or `nothing`), then ` (W/LIMIT)` with the carried weight.
- `go DIR`: `error: no exit DIR`, then `error: door DIR is locked`; otherwise the player moves and `-> ROOM` is printed.
- `take X`: X must be directly in the room (`error: no X here`); then if carried weight plus total(X) exceeds LIMIT, `error: too heavy`. Prints `taken X`.
- `drop X`: X must be carried directly (`error: not carrying X`). It lands in the room. Prints `dropped X`.
- `put X in B`: X and B must each be visible (`error: no X here` for X first, then B); B must be a box (`error: B is not a box`); B must not be X itself or lie inside X (`error: cannot put X in itself`); B must be open (`error: B is closed`); total weight of B's direct contents plus total(X) must not exceed B's CAP (`error: B is full`); finally, if B is carried and X was in the room, the carried weight plus total(X) must not exceed LIMIT (`error: too heavy`). Prints `put X in B`.
- `take X from B`: B visible (`error: no B here`), a box (`error: B is not a box`), open (`error: B is closed`); X must be directly inside B (`error: X is not in B`); if B is in the room (not carried) the weight check of `take` applies (`error: too heavy`). X becomes directly carried. Prints `taken X from B`.
- `open B` / `close B`: B visible (`error: no B here`), a box (`error: B is not a box`), not already in that state (`error: B is already open` / `error: B is already closed`). Prints `opened B` / `closed B`.
- `unlock DIR`: the exit must exist (`error: no exit DIR`) and be locked (`error: DIR is not locked`); the player must carry the door's KEY somewhere inside the carried items, at any depth and whether or not the boxes are open (`error: need KEY`). The door stays unlocked afterwards and the key is not consumed. Prints `unlocked DIR`.

Output is printed as commands run. The exit status is 1 if any printed line starts with `error:`, otherwise 0.

Example:

```
room hall
room attic
exit hall n attic
exit attic s hall
thing lamp 3 hall
box chest 10 5 hall open
player hall 5
---
take lamp
put lamp in chest
inv
take chest
look
go n
```

prints

```
taken lamp
put lamp in chest
carrying: nothing (0/5)
error: too heavy
== hall ==
exits: n
items: chest{lamp}
-> attic
```

and exits with status 1.
