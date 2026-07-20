use std::time::Instant;

fn min3(a: i32, b: i32, c: i32) -> i32 {
    if a <= b { if a <= c { a } else { c } }
    else { if b <= c { b } else { c } }
}

fn max3(a: i32, b: i32, c: i32) -> i32 {
    if a >= b { if a >= c { a } else { c } }
    else { if b >= c { b } else { c } }
}

fn mid3(a: i32, b: i32, c: i32) -> i32 {
    a + b + c - min3(a, b, c) - max3(a, b, c)
}

fn main() {
    let a = 9; let b = 3; let c = 7;
    let start = Instant::now();
    let lo = min3(a, b, c);
    let mi = mid3(a, b, c);
    let hi = max3(a, b, c);
    let elapsed = start.elapsed().as_nanos();
    println!("{} {} {} ({}ns)", lo, mi, hi, elapsed);
}
