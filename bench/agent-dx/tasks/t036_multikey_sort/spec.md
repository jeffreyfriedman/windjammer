The first line is a sort specification: comma-separated `field direction` pairs written `field:dir`, for example `age:desc,name:asc`. Fields are `name`, `age`, `city`; direction is `asc` or `desc`. The following lines are records `name age city` (three space-separated words; age is an integer).

Sort records by the first key, ties by the second, and so on; if all keys tie keep the original input order (stable). `name` and `city` compare as byte strings, `age` numerically. Print the sorted records unchanged, one per line.
