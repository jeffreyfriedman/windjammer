Implement a limit order book with price-time priority. The input holds one command per line, fields separated by single spaces; empty lines are ignored. Trader names are non-empty tokens. Numbers are digits only: QTY is 1 to 1000000, PRICE is 1 to 100000, ID is a positive integer with at most 7 digits. All output is printed as commands are processed.

**Line splitting (exact, no trimming).** Fields are obtained by splitting the line on single space characters, exactly. A leading, trailing or doubled space therefore produces an empty field, and the line is handled like any other line with a wrong number of fields or a malformed token (for example ` open a 1` is not `open a 1`). Only a line of length zero is an empty line: a line that holds only spaces is not.

Commands:

- `buy TRADER QTY PRICE [ioc|fok]` and `sell TRADER QTY PRICE [ioc|fok]` submit a limit order. Without a flag the order is good-till-cancelled.
- `mbuy TRADER QTY` and `msell TRADER QTY` submit a market order: it has no price limit and is treated like `ioc`.
- `cancel TRADER ID` and `amend TRADER ID QTY` change a resting order (QTY is a new quantity in the QTY range).
- `book` prints the book. `pos` prints the positions.

Anything else, or any argument outside the ranges, prints `error: bad command`. A malformed command does not consume an order id and does not create a trader.

Order ids. Every well-formed order command (buy, sell, mbuy, msell) gets the next id, 1, 2, 3, ... in input order, whether or not it later trades, rests or is rejected. The submitter becomes a known trader (for `pos`) as soon as the command is well formed.

Submitting an order with id N prints `order N` first. Then:

1. If the flag is `fok` (fill or kill) and the quantity that can trade, counting only resting orders of other traders that cross the limit, is less than QTY, print `reject N fok` and stop; nothing else changes.
2. Match against the opposite side. A resting order crosses a buy when its price is at most the buy's limit (any price for a market order), and a sell when its price is at least the sell's limit. Take resting orders best price first (lowest ask for a buy, highest bid for a sell) and, inside one price, the oldest first. For each candidate:
   - if it belongs to the same trader as the incoming order, it is removed from the book and `cancel ID self-trade` is printed (ID is the resting order's id); matching continues with the next candidate
   - otherwise trade `q = min(incoming remaining, resting remaining)` at the resting order's price and print `trade BUYID SELLID q @ PRICE` where BUYID and SELLID are the ids of the buy-side and sell-side orders. The buyer's position quantity grows by q and its cash falls by `q * PRICE`; the seller's position quantity falls by q and its cash grows by `q * PRICE`. A resting order whose quantity reaches 0 leaves the book.
   Matching stops when the incoming order is filled or nothing crosses any more.
3. If a quantity R is left: a good-till-cancelled limit order rests on the book (at its own limit price, behind older orders of that price) and `rest N R` is printed; an `ioc`, `fok` or market order is not rested and `drop N R` is printed. Nothing is printed when the order is completely filled.

Cancel and amend. The order must be resting on the book: otherwise `error: no such order ID` (also for ids never issued, filled, cancelled, or dropped). If it belongs to another trader, `error: not owner`. `cancel` removes the order and prints `cancelled ID R` where R is its remaining quantity. `amend` sets the remaining quantity to QTY and prints `amended ID QTY`; if QTY is greater than the current remaining quantity the order goes to the back of the queue at its price (it loses time priority), otherwise it keeps its place. Amend never triggers trading.

Book. `book` prints `asks:` then one line per ask price level from the highest price down to the lowest, then `bids:` then one line per bid level from the highest price down. A level line is two spaces, then `PRICE TOTALQTY ORDERS` (total remaining quantity at that price and the number of orders). An empty side shows `  (none)`.

Positions. `pos` prints `TRADER QTY CASH` for every known trader in ascending byte order of name (QTY and CASH are signed integers, starting at 0), or `(no traders)` if there are none.

Exit status: 1 if any printed line starts with `error:`, otherwise 0.

Example:

```
sell ann 10 101
sell bob 5 100
buy cy 12 101
book
pos
cancel ann 1
```

prints

```
order 1
rest 1 10
order 2
rest 2 5
order 3
trade 3 2 5 @ 100
trade 3 1 7 @ 101
asks:
  101 3 1
bids:
  (none)
ann -7 707
bob -5 500
cy 12 -1207
cancelled 1 3
```
