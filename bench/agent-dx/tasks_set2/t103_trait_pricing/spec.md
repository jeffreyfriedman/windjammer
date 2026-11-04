Each input line describes a purchase: `NAME QTY UNIT KIND`, where QTY and UNIT (price in cents) are non-negative integers and KIND is `plain`, `bulk` or `member`. Compute the line total in cents:

- `plain`: `QTY * UNIT`
- `bulk`: `QTY * UNIT`, minus 10% (integer division, rounding the discount down) when QTY >= 10
- `member`: `QTY * UNIT`, minus 5% (the discount rounded down); an additional flat 100 cents off when QTY >= 3, never going below 0

Print `NAME $D.CC` for each line, where D and CC are the dollars and two-digit cents of the total. A line with an unknown KIND prints `NAME invalid kind` and is not counted. Finally print `total $D.CC` with the sum of all counted lines. Blank lines are ignored.

Model the three pricing rules as a trait with three implementing types.
