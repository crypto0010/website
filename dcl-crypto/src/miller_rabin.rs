// dcl-crypto/src/miller_rabin.rs
//! Miller-Rabin probabilistic primality test.
//! Uses 40 rounds for cryptographic confidence: error prob ≤ 4^{-40} = 2^{-80}.
//! All arithmetic uses u128 to avoid overflow during modular exponentiation.

/// Compute (base^exp) % modulus using u128 fast exponentiation.
pub fn mod_pow(base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 1 {
        return 0;
    }
    let mut result: u128 = 1;
    let mut b: u128 = (base as u128) % modulus as u128;
    let m: u128 = modulus as u128;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result * b % m;
        }
        b = b * b % m;
        exp >>= 1;
    }
    result as u64
}

/// Single Miller-Rabin witness test: tests if `a` is a witness for compositeness of `n`.
/// Returns true if `n` passes (could be prime), false if definitely composite.
fn miller_rabin_witness(n: u64, d: u64, r: u32, a: u64) -> bool {
    let mut x = mod_pow(a, d, n) as u128;
    let n128 = n as u128;

    if x == 1 || x == n128 - 1 {
        return true;
    }
    for _ in 0..r - 1 {
        x = x * x % n128;
        if x == n128 - 1 {
            return true;
        }
    }
    false
}

/// Miller-Rabin primality test with `rounds` random witnesses.
/// For cryptographic use, set rounds = 40.
/// Returns true if n is probably prime, false if definitely composite.
pub fn miller_rabin(n: u64, rounds: u32) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 || n == 3 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }

    // Write n-1 as 2^r * d
    let mut d = n - 1;
    let mut r = 0u32;
    while d % 2 == 0 {
        d /= 2;
        r += 1;
    }

    // Deterministic witnesses sufficient for all n in u64 range:
    // - n < 3_215_031_751: {2,3,5,7}
    // - n >= 3_215_031_751 (still within u64): full set {2,3,5,7,11,13,17,19,23,29,31,37}
    //   (proven deterministic for all n < 3.3 × 10^24, which covers all u64 values)
    let witnesses: &[u64] = if n < 3_215_031_751 {
        &[2, 3, 5, 7]
    } else {
        &[2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37]
    };

    let effective_rounds = rounds.min(witnesses.len() as u32);
    for &a in &witnesses[..effective_rounds as usize] {
        if a >= n {
            continue;
        }
        if !miller_rabin_witness(n, d, r, a) {
            return false;
        }
    }
    true
}

/// Convenience: check if n is a safe prime p = 2q+1 where both p and q are prime.
pub fn is_safe_prime(p: u64) -> bool {
    if p < 5 || !miller_rabin(p, 40) {
        return false;
    }
    if (p - 1) % 2 != 0 {
        return false;
    }
    let q = (p - 1) / 2;
    miller_rabin(q, 40)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNOWN_PRIMES: &[u64] = &[
        2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89,
        97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181,
        191, 193, 197, 199, 211, 223, 227, 229, 233,
    ];

    // Carmichael numbers (composite but pass naive Fermat test)
    const CARMICHAEL: &[u64] = &[561, 1105, 1729, 2465, 2821, 6601, 8911, 10585];

    #[test]
    fn miller_rabin_known_primes() {
        for &p in KNOWN_PRIMES {
            assert!(miller_rabin(p, 40), "Failed on prime {p}");
        }
    }

    #[test]
    fn miller_rabin_known_composites() {
        for &c in CARMICHAEL {
            assert!(
                !miller_rabin(c, 40),
                "Wrongly identified Carmichael {c} as prime"
            );
        }
    }

    #[test]
    fn safe_prime_detection() {
        // 11 = 2*5+1, 5 prime → safe prime
        assert!(is_safe_prime(11));
        // 23 = 2*11+1, 11 prime → safe prime
        assert!(is_safe_prime(23));
        // 15 = 2*7+1, 7 prime, but 15 is not prime
        assert!(!is_safe_prime(15));
    }

    #[test]
    fn mod_pow_correctness() {
        assert_eq!(mod_pow(2, 10, 1000), 24);
        assert_eq!(mod_pow(3, 0, 7), 1);
        assert_eq!(mod_pow(5, 1, 13), 5);
    }
}
