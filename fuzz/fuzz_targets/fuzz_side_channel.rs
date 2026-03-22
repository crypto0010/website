#![no_main]

use libfuzzer_sys::fuzz_target;
use dcl_security::side_channel;

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 {
        return;
    }

    // Extract two u64 values for constant-time operations
    let a = u64::from_le_bytes([
        data[0], data[1], data[2], data[3],
        data[4], data[5], data[6], data[7],
    ]);
    let b = u64::from_le_bytes([
        data[8], data[9], data[10], data[11],
        data[12], data[13], data[14], data[15],
    ]);

    // Test constant-time GCD
    if a > 0 && b > 0 && a != u64::MAX && b != u64::MAX {
        let gcd = side_channel::gcd_constant_time(a, b);

        // Verify GCD properties
        assert!(gcd > 0, "GCD must be positive");
        assert!(gcd <= a.min(b), "GCD cannot exceed min(a,b)");

        // For small values, verify against standard algorithm
        if a < 1_000_000 && b < 1_000_000 {
            let expected_gcd = compute_gcd_reference(a, b);
            assert_eq!(gcd, expected_gcd, "Constant-time GCD must match reference implementation");
        }

        // Test coprimality check
        let coprime = side_channel::are_coprime_constant_time(a, b);
        assert_eq!(coprime, gcd == 1, "Coprime check must match gcd == 1");

        // Test determinism
        let gcd2 = side_channel::gcd_constant_time(a, b);
        assert_eq!(gcd, gcd2, "Constant-time GCD must be deterministic");
    }

    // Test constant-time byte comparison
    if data.len() >= 32 {
        let slice1 = &data[0..16];
        let slice2 = &data[16..32];

        let result1 = side_channel::compare_bytes_constant_time(slice1, slice2);
        let result2 = side_channel::compare_bytes_constant_time(slice1, slice2);

        // Determinism
        assert_eq!(result1, result2, "Byte comparison must be deterministic");

        // Self-equality
        let self_equal = side_channel::compare_bytes_constant_time(slice1, slice1);
        assert!(self_equal, "Byte array must equal itself");

        // Match with standard comparison
        let expected = slice1 == slice2;
        assert_eq!(result1, expected, "Constant-time comparison must match standard comparison");
    }

    // Test constant-time selection
    if data.len() >= 17 {
        let condition = data[16] % 2 == 0;
        let selected = side_channel::select_u64_constant_time(condition, a, b);

        let expected = if condition { a } else { b };
        assert_eq!(selected, expected, "Constant-time select must choose correctly");
    }

    // Test modular exponentiation resistance
    if data.len() >= 24 {
        let base = ((data[16] as u64) % 100) + 1;
        let exp = ((data[17] as u64) % 20) + 1;
        let modulus = ((data[18] as u64) % 100) + 2;

        if modulus > 1 {
            let result = side_channel::mod_pow_resistant(base, exp, modulus);

            // Verify result is less than modulus
            assert!(result < modulus, "Modular exponentiation result must be < modulus");

            // Test determinism
            let result2 = side_channel::mod_pow_resistant(base, exp, modulus);
            assert_eq!(result, result2, "Modular exponentiation must be deterministic");

            // Verify against reference for small values
            if base < 100 && exp < 10 && modulus < 100 {
                let expected = mod_pow_reference(base, exp, modulus);
                assert_eq!(result, expected, "Resistant mod_pow must match reference");
            }
        }
    }

    // Test cache-resistant operations
    if data.len() >= 32 {
        let test_data = &data[0..32];

        // Hash with blinding
        let hash = side_channel::cache_resistant::hash_with_blinding(test_data);
        assert_eq!(hash.len(), 32, "Hash must be 32 bytes");

        // Blinded memcmp
        let result = side_channel::cache_resistant::memcmp_blinded(test_data, test_data);
        assert!(result, "Blinded memcmp must return true for equal arrays");
    }

    // Test timing-resistant coprime batch check
    if data.len() >= 32 {
        let pairs = vec![
            (a, b),
            (a + 1, b + 1),
            (a % 100 + 1, b % 100 + 1),
        ];

        let results = side_channel::TimingResistantCoprime::batch_check(&pairs);
        assert_eq!(results.len(), 3, "Batch check must return result for each pair");

        // Verify individual results match
        for (i, &(x, y)) in pairs.iter().enumerate() {
            let individual = side_channel::TimingResistantCoprime::are_coprime(x, y);
            assert_eq!(results[i], individual, "Batch result must match individual check");
        }
    }
});

// Reference implementations for verification

fn compute_gcd_reference(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

fn mod_pow_reference(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 1 {
        return 0;
    }
    let mut result = 1u64;
    base %= modulus;
    while exp > 0 {
        if exp % 2 == 1 {
            result = (result as u128 * base as u128 % modulus as u128) as u64;
        }
        exp >>= 1;
        base = (base as u128 * base as u128 % modulus as u128) as u64;
    }
    result
}
