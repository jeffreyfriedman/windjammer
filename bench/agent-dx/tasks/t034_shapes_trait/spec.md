Model shapes with a common interface (trait / interface / abstract base) that exposes `name`, `corners` and `double_area` (twice the area, always an integer). Each input line describes one shape:

- `square S` : corners 4, double_area 2*S*S
- `rect W H` : corners 4, double_area 2*W*H
- `rtri A B` (right triangle with legs A, B) : corners 3, double_area A*B
- `trap A B H` (trapezoid with parallel sides A, B and height H) : corners 4, double_area (A+B)*H

For each shape print `<name> corners=<c> area2=<d>` where name is the first word. After all shapes print `total area2=<sum>` and `largest=<name> area2=<d>` for the first shape with the largest double_area (print `largest=none` if there are no shapes). All numbers are positive and fit in 64 bits.
