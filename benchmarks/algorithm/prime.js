function isPrime(n, d) {
    if (d * d > n) { return true }
    if (n % d == 0) { return false }
    return isPrime(n, d + 1)
}

function countPrimes(limit, cur) {
    if (cur > limit) { return 0 }
    let add = isPrime(cur, 2) ? 1 : 0
    return add + countPrimes(limit, cur + 1)
}

const n = 5000
const start = performance.now()
const result = countPrimes(n, 2)
const elapsed = (performance.now() - start).toFixed(0)
console.log(`primes under ${n} = ${result} (${elapsed}ms)`)
