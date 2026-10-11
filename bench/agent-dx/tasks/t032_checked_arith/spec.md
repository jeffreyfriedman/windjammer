Each input line is `op a b` with op one of `add sub mul pow div rem` and a, b signed integers that fit in 32 bits. Compute the result in signed 32-bit arithmetic WITHOUT wrapping. Print the result, or:
- `overflow` if the exact result does not fit in i32,
- `divzero` for `div`/`rem` with b = 0 (checked before overflow),
- `invalid` for `pow` with a negative exponent b.
`div` truncates toward zero, `rem` takes the sign of the dividend. `pow` computes a^b (0^0 = 1) and must not loop for huge exponents like 2000000000 when the base is 0, 1 or -1.
