let total = 0;
for (let i = 0; i < 5000000; i++) {
  const x = i * 3 + 1;
  if (x % 7 === 0) total += x; else total -= 1;
}
console.log(String(total));
