Simulate a vending machine. Stock: `cola` 150 cents (3 units), `chips` 100 cents (2 units), `gum` 35 cents (5 units). The machine accepts coins of 5, 10, 25, 100 cents. State: the credit inserted so far (starts 0) and the coin box (unlimited coins of each denomination are available for change). Commands:

- `coin V` : if V is an accepted denomination add to credit and print `credit <total>`, else print `rejected <V>` (credit unchanged).
- `select ITEM` : checks in this order: unknown item prints `error: unknown item <ITEM>`; sold out prints `error: sold out <ITEM>`; credit below the price prints `error: need <price - credit> more`. Otherwise dispense: the stock decreases by one, the credit resets to 0, and ONE line is printed: `dispensed <ITEM> change: <coins>` where coins are the change (credit - price) as greedy coins, largest first among 100, 25, 10, 5, separated by single spaces (for example `dispensed gum change: 25 25 10 5`), or the word `none` when the change is 0.
- `refund` : print `refund: <coins greedy>` (or `refund: none` if credit is 0), credit resets to 0.
- `status` : print `credit=<c> cola=<n> chips=<n> gum=<n>`.
Credit is always a multiple of 5 so greedy change is exact.
