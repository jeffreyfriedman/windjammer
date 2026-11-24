Simulate a small bank ledger. The input holds one command per line, fields separated by single spaces; empty lines are ignored. Account names are non-empty tokens without spaces. AMOUNT is digits only with at most 9 digits (cents). Output is produced as commands are processed.

**Line splitting (exact, no trimming).** Fields are obtained by splitting the line on single space characters, exactly. A leading, trailing or doubled space therefore produces an empty field, and the line is handled like any other line with a wrong number of fields or a malformed token (for example ` open a 1` is not `open a 1`). Only a line of length zero is an empty line: a line that holds only spaces is not.

Commands:

- `open NAME AMOUNT` creates an account with that opening balance (AMOUNT may be 0). If NAME already exists: `error: account exists NAME`.
- `transfer FROM TO AMOUNT` moves AMOUNT (at least 1) from one account to another and prints nothing. Checks, in this order: FROM unknown gives `error: unknown account FROM`; TO unknown gives `error: unknown account TO`; FROM equals TO gives `error: self transfer`; FROM's balance below AMOUNT gives `error: overdraft FROM`. A failed command changes nothing.
- `iou DEBTOR CREDITOR AMOUNT` records that DEBTOR owes CREDITOR AMOUNT (at least 1) in the IOU book and prints nothing. It moves no money. Checks in order: DEBTOR unknown, CREDITOR unknown (same messages as above), then `error: self transfer`.
- `balance NAME` prints `NAME: N` with the balance, or `error: unknown account NAME`.
- `all` prints `NAME: N` for every account in ascending byte order of name, or `(no accounts)` when there are none.
- `history NAME` prints the account's journal, one entry per line, in the order the entries happened, or `error: unknown account NAME`. The entries are `open N` (written once at creation), `in N from X` (money received from account X) and `out N to X` (money sent to account X). Both sides of every executed transfer get an entry; this includes transfers made by `settle`.
- `settle` clears the IOU book and pays it off with the following greedy procedure. First compute each account's net position: the total owed to it minus the total it owes. Accounts with a negative net are debtors, accounts with a positive net are creditors. Repeat until no debtor remains: pick the debtor with the largest debt (ties: smallest name) and the creditor with the largest credit (ties: smallest name), let `amt` be the smaller of the two, print `D pays C amt`, and reduce both positions by `amt`. If D's balance is at least `amt` the money moves (and the journal entries are written); otherwise nothing moves and the printed line ends with ` (failed: overdraft)`. The positions are reduced in both cases. If the IOU book yields no steps, print `nothing to settle`. The IOU book is empty afterwards in every case.

Any other line (unknown command, wrong number of fields, bad number, AMOUNT of 0 where at least 1 is required) prints `error: bad command`.

Exit status: 1 if at least one printed line starts with `error:`, otherwise 0. A `(failed: overdraft)` settle line does not count as an error line.

Example:

```
open ann 100
open bob 20
open cy 0
iou ann bob 30
iou bob cy 50
iou cy ann 10
settle
all
history bob
transfer cy ann 50
```

The net positions are ann -20 (owes 30, is owed 10), bob -20 and cy +40, so the output is

```
ann pays cy 20
bob pays cy 20
ann: 80
bob: 0
cy: 40
open 20
out 20 to cy
error: overdraft cy
```

and the exit status is 1.
