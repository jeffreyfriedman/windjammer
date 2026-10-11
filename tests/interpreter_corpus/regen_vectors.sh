#!/bin/bash
# regenerate committed vectors from the python reference models
cd "$1"
for f in *.ref.py; do
  n=${f%.ref.py}
  seeds=$(grep -m1 '^// seeds:' $n.wj | sed 's|// seeds: ||')
  rm -rf $n.vectors; mkdir -p $n.vectors
  for s in $seeds; do python3 -I $f $s > $n.vectors/$s.out; done
done
