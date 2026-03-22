#![no_main]

use libfuzzer_sys::fuzz_target;
use dcl_crypto::miller_rabin::miller_rabin;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    // Extract candidate number from fuzzer data
    let n = u64::from_le_bytes([
        data[0], data[1], data[2], data[3],
        data[4], data[5], data[6], data[7],
    ]);

    // Skip trivial cases
    if n < 2 || n == u64::MAX {
        return;
    }

    // Extract number of rounds (1-64)
    let rounds = if data.len() > 8 {
        ((data[8] as u32) % 64) + 1
    } else {
        5
    };

    // Test Miller-Rabin primality test
    let is_probably_prime = miller_rabin(n, rounds);

    // Known composite numbers should always return false
    if n % 2 == 0 && n > 2 {
        assert!(!is_probably_prime, "Even numbers > 2 must be composite");
    }

    // Known primes should always return true
    let known_primes = [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];
    if known_primes.contains(&n) {
        assert!(is_probably_prime, "Known primes must return true");
    }

    // Known composites should always return false
    let known_composites = [4u64, 6, 8, 9, 10, 12, 14, 15, 16, 18, 20, 21, 22, 24, 25];
    if known_composites.contains(&n) {
        assert!(!is_probably_prime, "Known composites must return false");
    }

    // Test deterministic behavior (same input -> same output)
    let result1 = miller_rabin(n, rounds);
    let result2 = miller_rabin(n, rounds);
    assert_eq!(result1, result2, "Miller-Rabin must be deterministic for same input");

    // More rounds should not change true primes to composites
    if is_probably_prime && rounds < 32 {
        let more_rounds_result = miller_rabin(n, rounds + 10);
        // If it was prime with fewer rounds, it should still be prime with more rounds
        // (though this isn't strictly guaranteed for probabilistic tests)
        if known_primes.contains(&n) {
            assert!(more_rounds_result, "Known primes must remain prime with more rounds");
        }
    }

    // Test powers of 2 (all composite except 2)
    if n.is_power_of_two() && n > 2 {
        assert!(!is_probably_prime, "Powers of 2 (except 2) must be composite");
    }

    // Test small safe primes if we have one
    // Safe prime p where (p-1)/2 is also prime
    if is_probably_prime && n > 2 && n % 2 == 1 {
        let sophie_germain = (n - 1) / 2;
        if sophie_germain > 1 {
            let _ = miller_rabin(sophie_germain, rounds);
        }
    }

    // Carmichael numbers (pseudoprimes) should eventually be detected as composite
    // 561 = 3 × 11 × 17 is the smallest Carmichael number
    if n == 561 {
        // With enough rounds, should detect as composite
        if rounds >= 20 {
            assert!(!is_probably_prime, "Carmichael number 561 should be detected as composite");
        }
    }
});
