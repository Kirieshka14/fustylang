// FustyLang Native x86-64 JIT Benchmark (~nya edition)
// Run with: cargo run --release -p fusty_cli -- bench examples/jit_benchmark.fy

nya fib(n: i64) -> i64 {
    if n <= 1 {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

nya sum_squares(limit: i64) -> i64 {
    let mut acc = 0;
    let mut i = 1;
    while i <= limit {
        acc += i * i;
        i += 1;
    }
    return acc;
}

let f = fib(24);
let s = sum_squares(50000);
return f + s;
