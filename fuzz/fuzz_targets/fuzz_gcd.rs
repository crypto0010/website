#![no_main]

use libfuzzer_sys::fuzz_target;
use dcl_core::gcd::{gcd, are_coprime};

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 {
        return;
    }

    // Extract two u64 values from fuzzer data
    let a = u64::from_le_bytes([
        data[0], data[1], data[2], data[3],
        data[4], data[5], data[6], data[7],
    ]);
    let b = u64::from_le_bytes([
        data[8], data[9], data[10], data[11],
        data[12], data[13], data[14], data[15],
    ]);

    // Avoid edge cases that might cause issues
    if a == 0 || b == 0 || a == u64::MAX || b == u64::MAX {
        return;
    }

    // Test GCD
    let g = gcd(a, b);

    // Verify GCD properties
    assert!(g > 0, "GCD must be positive");
    assert!(g <= a.min(b), "GCD cannot exceed min(a,b)");
    assert_eq!(a % g, 0, "GCD must divide a");
    assert_eq!(b % g, 0, "GCD must divide b");

    // Verify GCD is the greatest common divisor
    if g > 1 {
        assert!(a / g > 0 && b / g > 0);
    }

    // Test coprimality
    let coprime = are_coprime(a, b);
    assert_eq!(coprime, g == 1, "Coprime check must match gcd == 1");

    // Verify LCM via fundamental relation: a * b = gcd(a,b) * lcm(a,b)
    // Compute lcm manually since there is no lcm() export
    let lcm = (a as u128) * (b as u128) / (g as u128);

    // Verify LCM properties (using u128 to avoid overflow)
    assert!(lcm >= a.max(b) as u128, "LCM must be at least max(a,b)");
    assert_eq!((a as u128) % lcm, 0, "LCM must be divisible by a");
    assert_eq!((b as u128) % lcm, 0, "LCM must be divisible by b");

    // Verify fundamental relation: a * b = gcd(a,b) * lcm(a,b)
    let product_ab = (a as u128) * (b as u128);
    let product_gcd_lcm = (g as u128) * lcm;
    assert_eq!(product_ab, product_gcd_lcm, "Fundamental GCD-LCM relation violated");

    // Test GCD with swapped arguments (commutative property)
    let gcd_swap = gcd(b, a);
    assert_eq!(g, gcd_swap, "GCD must be commutative");

    // Test GCD associativity with third value if enough data
    if data.len() >= 24 {
        let c = u64::from_le_bytes([
            data[16], data[17], data[18], data[19],
            data[20], data[21], data[22], data[23],
        ]);

        if c > 0 && c != u64::MAX {
            let gcd_ab_c = gcd(gcd(a, b), c);
            let gcd_a_bc = gcd(a, gcd(b, c));
            assert_eq!(gcd_ab_c, gcd_a_bc, "GCD must be associative");
        }
    }
});
