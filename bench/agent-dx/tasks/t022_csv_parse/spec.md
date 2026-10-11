Each input line is one CSV record. Fields are separated by commas. A field may be wrapped in double quotes, in which case it may contain commas, and a doubled quote `""` inside it stands for a literal `"`. Quotes only appear at the very start/end of a field; the input is always well formed. An empty line is a record with one empty field. No field contains a newline.

For each record print `<n>: [f1]|[f2]|...` where n is the number of fields and each field value (quotes removed, `""` unescaped) is wrapped in square brackets.
