Track a warehouse inventory. Each item has a SKU (a single word), a quantity (non-negative integer) and a unit price in cents (positive integer, set when the SKU is first stocked). Commands, one per line:

- `stock SKU QTY PRICE` : add QTY units. First, QTY must be > 0 else print `error: bad quantity`. Then, if the SKU is new it is created with PRICE; if it exists PRICE must equal its price, otherwise print `error: price mismatch for <SKU>` and change nothing. On success print `ok <new quantity>`.
- `sell SKU QTY` : remove QTY units (QTY > 0 else `error: bad quantity`). Unknown SKU prints `error: unknown sku <SKU>`; too few units prints `error: insufficient stock for <SKU> (have <h>, need <n>)`. On success print `sold <QTY> for <QTY*price>` (cents).
- `remove SKU` : delete the SKU if its quantity is 0 (print `removed SKU`); `error: unknown sku SKU` if absent; `error: not empty SKU` if units remain.
- `report` : print one line per SKU sorted by SKU: `<sku> <qty> x <price> = <value>`, then `total value <sum>` (sum of qty*price over SKUs). If there are no SKUs print only `(empty)`.
Checks happen in the order described for each command (for `sell`: quantity, then unknown SKU, then stock).
