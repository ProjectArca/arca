function fib(n) {
  if (n <= 1) return n;
  return fib(n - 1) + fib(n - 2);
}

const n = 45;
const start = Date.now();
const result = fib(n);
const elapsed = Date.now() - start;
console.log(`Bun fib(${n}) = ${result} (${elapsed}ms)`);
