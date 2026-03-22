//! Generate .mem test vector files for FPGA testbenches.
//! Run: rustc generate_test_vectors.rs -o gen_vectors && ./gen_vectors
//! Or add as a binary to dcl_fpga_host.

use std::fs::File;
use std::io::Write;

fn gcd(mut a: u64, mut b: u64) -> u64 {
    if a == 0 { return b; }
    if b == 0 { return a; }
    let shift = (a | b).trailing_zeros();
    a >>= a.trailing_zeros();
    loop {
        b >>= b.trailing_zeros();
        if a > b { std::mem::swap(&mut a, &mut b); }
        b -= a;
        if b == 0 { return a << shift; }
    }
}

fn main() {
    // GCD test vectors: a, b, expected_gcd
    let gcd_cases: Vec<(u64, u64, u64)> = vec![
        (12, 8, 4), (17, 13, 1), (0, 5, 5), (100, 0, 100),
        (0, 0, 0), (1, 1, 1), (7, 13, 1), (6, 9, 3),
        (1024, 512, 512), (104729, 104743, 1),
    ];

    let mut f = File::create("gcd_vectors.mem").unwrap();
    for (a, b, expected) in &gcd_cases {
        assert_eq!(gcd(*a, *b), *expected);
        writeln!(f, "{:016x} {:016x} {:016x}", a, b, expected).unwrap();
    }
    println!("Generated gcd_vectors.mem ({} cases)", gcd_cases.len());
}
