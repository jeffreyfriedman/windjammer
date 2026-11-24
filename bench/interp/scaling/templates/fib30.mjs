function fib(n) {
  if (n < 2) return n;
  return fib(n - 1) + fib(n - 2);
}
const t0 = process.hrtime.bigint();
const r = fib(@n@);
console.error(`STEADY_NS ${process.hrtime.bigint() - t0}`);
console.log(String(r));
