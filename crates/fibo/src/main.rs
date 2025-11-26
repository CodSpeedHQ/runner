fn fibo_rec(n: u32) -> u32 {
    if n <= 1 {
        n
    } else {
        fibo_rec(n - 1) + fibo_rec(n - 2)
    }
}

fn main() {
    let n = 32;
    println!("Fibonacci of {} is {}", n, fibo_rec(n));
}
