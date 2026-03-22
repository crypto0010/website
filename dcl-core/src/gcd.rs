// dcl-core/src/gcd.rs
//! GCD utilities used throughout the DCL framework.
//! Uses Stein's (binary GCD) algorithm for efficiency.

/// Compute gcd(a, b) using the binary (Stein) algorithm.
/// Returns 0 if both inputs are 0.
pub fn gcd(mut a: u64, mut b: u64) -> u64 {
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

/// Returns true iff gcd(a, b) == 1.
#[inline]
pub fn are_coprime(a: u64, b: u64) -> bool {
    gcd(a, b) == 1
}

/// Compute gcd of a slice of values (setwise GCD).
/// Returns 0 for empty slices.
pub fn setwise_gcd(values: &[u64]) -> u64 {
    values.iter().copied().reduce(gcd).unwrap_or(0)
}

/// Extended Euclidean algorithm.
/// Returns (g, x, y) such that a*x + b*y = g = gcd(a, b).
pub fn extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    if b == 0 {
        return (a, 1, 0);
    }
    let (g, x, y) = extended_gcd(b, a % b);
    (g, y, x - (a / b) * y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcd_basic() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(17, 13), 1);
        assert_eq!(gcd(0, 5), 5);
        assert_eq!(gcd(100, 0), 100);
    }

    #[test]
    fn coprime_basic() {
        assert!(are_coprime(7, 13));
        assert!(!are_coprime(6, 9));
    }

    #[test]
    fn setwise_gcd_basic() {
        assert_eq!(setwise_gcd(&[12, 8, 4]), 4);
        assert_eq!(setwise_gcd(&[7, 11, 13]), 1);
        assert_eq!(setwise_gcd(&[]), 0);
    }

    #[test]
    fn extended_gcd_basic() {
        let (g, x, y) = extended_gcd(35, 15);
        assert_eq!(g, 5);
        assert_eq!(35 * x + 15 * y, g);
    }
}
