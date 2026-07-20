#!/usr/bin/env node
// HTTP benchmark client using Bun/Node built-in fetch

const TARGET = process.argv[2] || "http://localhost:3000";
const REQUESTS = parseInt(process.argv[3]) || 10000;
const CONCURRENCY = parseInt(process.argv[4]) || 100;

async function worker() {
  while (counter < REQUESTS) {
    const i = counter++;
    try {
      const res = await fetch(TARGET);
      await res.json();
    } catch (e) {
      errors++;
    }
  }
}

let counter = 0;
let errors = 0;
const start = Date.now();

const workers = Array.from({ length: CONCURRENCY }, () => worker());
await Promise.all(workers);

const elapsed = Date.now() - start;
const rps = Math.round((REQUESTS / elapsed) * 1000);
console.log(`\nResults:`);
console.log(`  Target:     ${TARGET}`);
console.log(`  Requests:   ${REQUESTS}`);
console.log(`  Concurrency: ${CONCURRENCY}`);
console.log(`  Duration:   ${elapsed}ms`);
console.log(`  RPS:        ${rps}`);
console.log(`  Errors:     ${errors}`);
