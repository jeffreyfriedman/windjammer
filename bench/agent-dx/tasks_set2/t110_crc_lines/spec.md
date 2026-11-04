For every line of standard input print the CRC-32 checksum (IEEE 802.3, the one used by zip and gzip) of the line's bytes without the line terminator, as exactly 8 lowercase hexadecimal digits with leading zeros. After the last line print `lines: N` with the number of lines.

Use the Rust crate `crc32fast`: the workspace already contains a `wj.toml` that declares it. Call its `hash` function (it takes the bytes of the line and returns the checksum as a 32-bit unsigned integer). Compiled programs use the crate; `wj run --compiled main.wj` runs it.
