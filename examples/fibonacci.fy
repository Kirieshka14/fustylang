// Fibonacci sequence in FustyLang (recursive & iterative)

fn fib_rec(n: i64) -> i64 {
    if n <= 1 {
        return n;
    }
    return fib_rec(n - 1) + fib_rec(n - 2);
}

fn fib_iter(n: i64) -> i64 {
    if n <= 1 {
        return n;
    }
    let mut a = 0;
    let mut b = 1;
    let mut i = 2;
    while i <= n {
        let next = a + b;
        a = b;
        b = next;
        i += 1;
    }
    return b;
}

log("fib_rec(10) =", fib_rec(10));
log("fib_iter(10) =", fib_iter(10));
