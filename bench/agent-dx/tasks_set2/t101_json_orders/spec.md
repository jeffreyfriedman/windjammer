Standard input is a JSON array of orders. Each order is an object with exactly these fields: `"id"` (integer), `"customer"` (string), `"items"` (array of objects with `"sku"` string, `"qty"` integer, `"price"` number) and optionally `"note"` (string; may be absent). Unknown fields never occur.

For every customer print one line `NAME TOTAL`, where TOTAL is the sum of `qty * price` over all items of all of that customer's orders, formatted with exactly two decimals. Customers are listed in ascending byte order of name. Customers with no items still appear with `0.00`.

If the input is not valid JSON for this shape (including empty input), print `invalid input` and exit with status 1. Otherwise exit 0.
