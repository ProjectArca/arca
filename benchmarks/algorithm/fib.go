package main

import (
    "fmt"
    "time"
)

func fib(n int64) int64 {
    if n <= 1 { return n }
    return fib(n - 1) + fib(n - 2)
}

func main() {
    var n int64 = 45
    start := time.Now()
    result := fib(n)
    elapsed := time.Since(start).Milliseconds()
    fmt.Printf("Go fib(%d) = %d (%dms)\n", n, result, elapsed)
}
