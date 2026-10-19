let s = "";
let total = 0;
for (let i = 0; i < 200000; i++) {
  s += `${i % 10},`;
  if (s.length > 1000) { total += s.length; s = ""; }
}
console.log(`${total} ${s.length}`);
