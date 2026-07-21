package main

import (
    "fmt"
    "time"
)

func isPrime(n, d int) bool {
    if d*d > n { return true }
    if n%d == 0 { return false }
    return isPrime(n, d+1)
}

func countPrimes(limit, cur int) int {
    if cur > limit { return 0 }
    add := 0
    if isPrime(cur, 2) { add = 1 }
    return add + countPrimes(limit, cur+1)
}

func main() {
    n := 10000
    start := time.Now()
    result := countPrimes(n, 2)
    elapsed := time.Since(start).Milliseconds()
    fmt.Printf("primes under %d = %d (%dms)\n", n, result, elapsed)
}
