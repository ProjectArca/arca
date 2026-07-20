use std::time::Instant;

fn fib(n: i64) -> i64 {
    if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
}

fn main() {
    let n = 45;
    let start = Instant::now();
    let result = fib(n);
    let elapsed = start.elapsed().as_millis();
    println!("Rust fib({}) = {} ({}ms)", n, result, elapsed);
}
