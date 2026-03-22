// dcl-ramsey/src/carmichael.rs
//! Carmichael function λ(n) and label orbit period analysis.
//! From Dynamic Prime Labeling ver1, §5:
//! Label evolution under g(x) = x^m is periodic with period dividing λ(n).

use dcl_core::gcd::gcd;

/// Compute Euler's totient φ(n).
pub fn euler_totient(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut result = n;
    let mut p = 2u64;
    let mut m = n;
    while p * p <= m {
        if m % p == 0 {
            while m % p == 0 {
                m /= p;
            }
            result -= result / p;
        }
        p += 1;
    }
    if m > 1 {
        result -= result / m;
    }
    result
}

/// Compute the Carmichael function λ(n) (reduced totient).
/// λ(n) is the exponent of (Z/nZ)*.
pub fn carmichael_lambda(n: u64) -> u64 {
    if n == 1 {
        return 1;
    }
    let factors = prime_power_factors(n);
    let lambdas: Vec<u64> = factors
        .iter()
        .map(|&(p, k)| {
            if p == 2 {
                match k {
                    1 => 1,
                    2 => 2,
                    _ => phi_prime_power(p, k) / 2,
                }
            } else {
                phi_prime_power(p, k)
            }
        })
        .collect();
    lambdas.iter().fold(1u64, |acc, &l| lcm(acc, l))
}

fn phi_prime_power(p: u64, k: u32) -> u64 {
    p.pow(k - 1) * (p - 1)
}

fn lcm(a: u64, b: u64) -> u64 {
    a / gcd(a, b) * b
}

/// Factorize n into (prime, exponent) pairs.
pub fn prime_power_factors(mut n: u64) -> Vec<(u64, u32)> {
    let mut factors = Vec::new();
    let mut p = 2u64;
    while p * p <= n {
        if n % p == 0 {
            let mut k = 0u32;
            while n % p == 0 {
                k += 1;
                n /= p;
            }
            factors.push((p, k));
        }
        p += 1;
    }
    if n > 1 {
        factors.push((n, 1));
    }
    factors
}

/// Modular exponentiation.
fn mod_pow(base: u64, mut exp: u64, modulus: u64) -> u64 {
    let mut result: u128 = 1;
    let mut b: u128 = (base % modulus) as u128;
    let m = modulus as u128;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result * b % m;
        }
        b = b * b % m;
        exp >>= 1;
    }
    result as u64
}

/// Compute orbit period of x^m mod n (modular — always periodic).
pub fn modular_label_period(start: u64, m: u32, n: u64) -> usize {
    if n <= 1 {
        return 1;
    }
    let mut x = start % n;
    let initial = x;
    for step in 1..=100_000usize {
        x = mod_pow(x, m as u64, n);
        if x == initial {
            return step;
        }
    }
    100_001
}

/// Compute orbit period of x under integer power map g(x) = x^m (non-modular).
/// Returns None if labels diverge beyond u64.
pub fn label_orbit_period(start: u64, m: u32, max_steps: usize) -> Option<usize> {
    if m == 1 {
        return Some(1);
    } // Identity — period 1
    if start <= 1 {
        return Some(1);
    } // 0 or 1 are fixed points
      // For m>=2, x grows without bound for x>1 — no finite period
      // (used only as a theoretical check for very small labels)
    let mut x = start;
    let mut seen = std::collections::HashMap::new();
    for step in 0..max_steps {
        if let Some(prev) = seen.get(&x) {
            return Some(step - prev);
        }
        seen.insert(x, step);
        let mut next: u128 = 1;
        let b = x as u128;
        for _ in 0..m {
            next = next.saturating_mul(b);
        }
        if next > u64::MAX as u128 {
            return None;
        }
        x = next as u64;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carmichael_known_values() {
        assert_eq!(carmichael_lambda(1), 1);
        assert_eq!(carmichael_lambda(2), 1);
        assert_eq!(carmichael_lambda(4), 2);
        assert_eq!(carmichael_lambda(7), 6);
        assert_eq!(carmichael_lambda(11), 10);
        assert_eq!(carmichael_lambda(13), 12);
    }

    #[test]
    fn euler_totient_known() {
        assert_eq!(euler_totient(1), 1);
        assert_eq!(euler_totient(6), 2);
        assert_eq!(euler_totient(9), 6);
        assert_eq!(euler_totient(10), 4);
    }

    #[test]
    fn modular_period_divides_lambda() {
        // By Carmichael's theorem, ord_n(a) | λ(n) when gcd(a,n)=1
        let n = 100u64;
        let lam = carmichael_lambda(n);
        for a in [3u64, 7, 11, 13, 17] {
            if gcd(a, n) == 1 {
                let period = modular_label_period(a, 1, n) as u64;
                assert_eq!(
                    lam % period,
                    0,
                    "ord({a} mod {n})={period} should divide λ({n})={lam}"
                );
            }
        }
    }
}
