const n = 1000000;
const flags = new Array(n).fill(true);
let count = 0;
for (let i = 2; i < n; i++) {
  if (flags[i]) {
    count++;
    for (let j = i * i; j < n; j += i) flags[j] = false;
  }
}
console.log(String(count));
