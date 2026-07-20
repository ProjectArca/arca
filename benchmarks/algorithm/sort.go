package main

import (
    "fmt"
    "time"
)

func min3(a, b, c int) int {
    if a <= b {
        if a <= c { return a } else { return c }
    } else {
        if b <= c { return b } else { return c }
    }
}

func max3(a, b, c int) int {
    if a >= b {
        if a >= c { return a } else { return c }
    } else {
        if b >= c { return b } else { return c }
    }
}

func mid3(a, b, c int) int {
    return a + b + c - min3(a, b, c) - max3(a, b, c)
}

func main() {
    a, b, c := 9, 3, 7
    start := time.Now()
    lo := min3(a, b, c)
    mi := mid3(a, b, c)
    hi := max3(a, b, c)
    elapsed := time.Since(start).Nanoseconds()
    fmt.Printf("%d %d %d (%dns)\n", lo, mi, hi, elapsed)
}
