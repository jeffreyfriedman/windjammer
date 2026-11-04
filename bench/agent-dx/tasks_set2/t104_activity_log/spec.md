Each input line is `USER ACTION SECONDS` (three space-separated tokens; SECONDS is a non-negative integer). Lines that do not have exactly three tokens or whose SECONDS is not a non-negative integer are counted as malformed and otherwise ignored.

For every user print `USER COUNT TOTAL TOP` where COUNT is the number of valid lines, TOTAL the sum of SECONDS and TOP the action that user performed most often (ties: the alphabetically smallest action). Users are ordered by TOTAL descending, then by name ascending. After the users print `malformed: N` (always printed, also when 0).
