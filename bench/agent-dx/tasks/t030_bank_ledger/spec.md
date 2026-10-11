Simulate a bank. Commands, one per line (amounts are positive integers of cents; names are single words):

- `open NAME AMOUNT` : create an account with an initial balance (AMOUNT may be 0).
- `deposit NAME AMOUNT`, `withdraw NAME AMOUNT`
- `transfer FROM TO AMOUNT`
- `balance NAME`

Success prints `ok` (for `balance` print the balance). Failures print one error line and change nothing, checked in this order: `error: invalid amount` (AMOUNT <= 0 for deposit/withdraw/transfer; negative for open), `error: duplicate account` (open of an existing name), `error: unknown account NAME` (first missing name, FROM before TO), `error: insufficient funds` (withdraw/transfer exceeding the balance), `error: same account` (transfer with FROM == TO, checked after unknown account but before insufficient funds).

After all commands print every account as `NAME BALANCE`, sorted by name ascending.
