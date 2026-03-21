//! GPU/CPU backend abstraction layer for DCL-Web.
//!
//! Wraps dcl-gpu CUDA kernels and falls back to dcl-core CPU functions
//! when no GPU is available.

use dcl_core::gcd::{gcd, are_coprime};
use dcl_crypto::miller_rabin::miller_rabin;
use dcl_gpu::CudaContext;
use dcl_gpu::kernels::gcd::GpuGcdBatch;
use dcl_gpu::kernels::power_map::GpuPowerMap;
use dcl_gpu::kernels::evolve::GpuEvolve;
use serde::Serialize;
use std::sync::Arc;

// ── Public types ─────────────────────────────────────────────────────────────

/// Result of a single GCD computation.
#[derive(Debug, Clone, Serialize)]
pub struct GcdResult {
    pub a: u64,
    pub b: u64,
    pub gcd: u64,
    pub coprime: bool,
}

/// Information about the compute backend currently in use.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub gpu_available: bool,
    pub device_name: String,
    pub backend: String,
}

// ── Backend struct ────────────────────────────────────────────────────────────

/// DCL compute backend. Prefers GPU; silently falls back to CPU.
pub struct DclBackend {
    gpu_ctx: Option<Arc<CudaContext>>,
    device_name: String,
}

impl DclBackend {
    /// Create a new backend.  Attempts to initialise a CUDA context on device 0.
    /// On failure (no GPU or driver not present) silently switches to CPU mode.
    pub fn new() -> Self {
        match CudaContext::new() {
            Ok(ctx) => {
                let name = ctx
                    .device_info()
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|_| "CUDA GPU".to_string());
                DclBackend {
                    gpu_ctx: Some(Arc::new(ctx)),
                    device_name: name,
                }
            }
            Err(_) => DclBackend {
                gpu_ctx: None,
                device_name: "CPU".to_string(),
            },
        }
    }

    /// Returns `"gpu"` or `"cpu"` depending on what is available.
    pub fn backend_name(&self) -> &str {
        if self.gpu_ctx.is_some() { "gpu" } else { "cpu" }
    }

    /// Returns structured information about the current device.
    pub fn device_info(&self) -> DeviceInfo {
        DeviceInfo {
            gpu_available: self.gpu_ctx.is_some(),
            device_name: self.device_name.clone(),
            backend: self.backend_name().to_string(),
        }
    }

    // ── Batch GCD ────────────────────────────────────────────────────────────

    /// Compute GCD for every pair (a[i], b[i]).  GPU-accelerated when possible.
    pub fn batch_gcd(&self, a: &[u64], b: &[u64]) -> Vec<GcdResult> {
        assert_eq!(a.len(), b.len(), "Input slices must have equal length");

        // GPU path
        if let Some(ctx) = &self.gpu_ctx {
            if let Ok(kernel) = GpuGcdBatch::new(ctx.as_ref().clone()) {
                if let Ok((gcds, coprimes)) = kernel.compute_batch(a, b) {
                    return a
                        .iter()
                        .zip(b.iter())
                        .zip(gcds.iter().zip(coprimes.iter()))
                        .map(|((&ai, &bi), (&g, &c))| GcdResult {
                            a: ai,
                            b: bi,
                            gcd: g,
                            coprime: c == 1,
                        })
                        .collect();
                }
            }
        }

        // CPU fallback
        a.iter()
            .zip(b.iter())
            .map(|(&ai, &bi)| {
                let g = gcd(ai, bi);
                GcdResult { a: ai, b: bi, gcd: g, coprime: g == 1 }
            })
            .collect()
    }

    // ── Power map ─────────────────────────────────────────────────────────────

    /// Apply the power map f(x) = x^m (optionally mod `modulus`) to every label.
    ///
    /// - `modulus = Some(n)`: compute x^m mod n.
    /// - `modulus = None`: unbounded, saturating at `u64::MAX`.
    pub fn power_map(&self, labels: &[u64], m: u32, modulus: Option<u64>) -> Vec<u64> {
        let mod_val = modulus.unwrap_or(0);

        // GPU path
        if let Some(ctx) = &self.gpu_ctx {
            if let Ok(kernel) = GpuPowerMap::new(ctx.as_ref().clone()) {
                if let Ok(result) = kernel.apply_batch(labels, m, mod_val) {
                    return result;
                }
            }
        }

        // CPU fallback
        labels.iter().map(|&x| cpu_power_map(x, m, modulus)).collect()
    }

    // ── Coprime check ─────────────────────────────────────────────────────────

    /// For each edge (u, v) check whether labels[u] and labels[v] are coprime.
    /// Returns a `Vec<bool>` parallel to `edges`.
    pub fn coprime_check(&self, labels: &[u64], edges: &[(usize, usize)]) -> Vec<bool> {
        edges
            .iter()
            .map(|&(u, v)| are_coprime(labels[u], labels[v]))
            .collect()
    }

    // ── Evolve ────────────────────────────────────────────────────────────────

    /// Apply the power map `steps` times in succession.
    ///
    /// GPU: delegates to `GpuEvolve::evolve_steps`.
    /// CPU: repeatedly calls `cpu_power_map`.
    pub fn evolve(&self, labels: &[u64], m: u32, modulus: Option<u64>, steps: usize) -> Vec<u64> {
        let mod_val = modulus.unwrap_or(0);

        // GPU path
        if let Some(ctx) = &self.gpu_ctx {
            if let Ok(kernel) = GpuEvolve::new(ctx.as_ref().clone()) {
                if let Ok(result) = kernel.evolve_steps(labels, m, mod_val, steps) {
                    return result;
                }
            }
        }

        // CPU fallback — iterate in-place
        let mut current: Vec<u64> = labels.to_vec();
        for _ in 0..steps {
            current = current.iter().map(|&x| cpu_power_map(x, m, modulus)).collect();
        }
        current
    }

    // ── Coprime repair ────────────────────────────────────────────────────────

    /// Repair non-coprime edges by incrementing the smaller label until the pair
    /// becomes coprime.  Returns the (possibly modified) label vector together
    /// with a list of edges that required repair.
    pub fn coprime_repair(
        &self,
        labels: &[u64],
        edges: &[(usize, usize)],
    ) -> (Vec<u64>, Vec<(usize, usize)>) {
        let mut repaired = labels.to_vec();
        let mut repaired_edges = Vec::new();

        for &(u, v) in edges {
            if !are_coprime(repaired[u], repaired[v]) {
                repaired_edges.push((u, v));
                // Increment the smaller endpoint until coprime.
                let (small, large) = if repaired[u] <= repaired[v] { (u, v) } else { (v, u) };
                while !are_coprime(repaired[small], repaired[large]) {
                    repaired[small] = repaired[small].saturating_add(1);
                }
            }
        }

        (repaired, repaired_edges)
    }

    // ── Safe prime sieve ──────────────────────────────────────────────────────

    /// Find up to `count` safe primes starting the search at `start`.
    ///
    /// A safe prime p satisfies: both p and (p-1)/2 are prime.
    ///
    /// Uses Miller-Rabin primality testing (40 rounds).
    pub fn prime_sieve(&self, start: u64, count: usize) -> Vec<u64> {
        let mut results = Vec::with_capacity(count);
        let mut candidate = if start < 5 { 5 } else { start | 1 }; // start odd, ≥ 5

        while results.len() < count {
            if miller_rabin(candidate, 40) {
                let sophie = (candidate - 1) / 2;
                if miller_rabin(sophie, 40) {
                    results.push(candidate);
                }
            }
            candidate = candidate.saturating_add(2);
            if candidate == u64::MAX {
                break;
            }
        }

        results
    }
}

// ── CPU helper ────────────────────────────────────────────────────────────────

/// CPU binary exponentiation for a single value.
///
/// - `modulus = Some(n)`: returns x^m mod n (result ≥ 1, clamped to 1 when 0).
/// - `modulus = None`:   unbounded, saturates at `u64::MAX` on overflow.
fn cpu_power_map(x: u64, m: u32, modulus: Option<u64>) -> u64 {
    match modulus {
        Some(n) => {
            if n == 0 {
                return x;
            }
            let n128 = n as u128;
            let mut result: u128 = 1;
            let mut base: u128 = (x as u128) % n128;
            let mut exp = m;
            while exp > 0 {
                if exp & 1 == 1 {
                    result = result * base % n128;
                }
                base = base * base % n128;
                exp >>= 1;
            }
            let v = result as u64;
            if v == 0 { 1 } else { v }
        }
        None => {
            if x <= 1 {
                return x;
            }
            let mut result: u64 = 1;
            let mut base: u64 = x;
            let mut exp = m;
            while exp > 0 {
                if exp & 1 == 1 {
                    result = match result.checked_mul(base) {
                        Some(v) => v,
                        None => return u64::MAX,
                    };
                }
                exp >>= 1;
                if exp > 0 {
                    base = match base.checked_mul(base) {
                        Some(v) => v,
                        None => return u64::MAX,
                    };
                }
            }
            result
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let backend = DclBackend::new();
        // Must always produce a valid backend name.
        let name = backend.backend_name();
        assert!(name == "gpu" || name == "cpu");

        let info = backend.device_info();
        assert_eq!(info.backend, name);
        assert!(!info.device_name.is_empty());
    }

    #[test]
    fn test_cpu_gcd() {
        let backend = DclBackend::new();
        let a = vec![12u64, 35, 17, 100];
        let b = vec![8u64,  25, 13,  75];
        let results = backend.batch_gcd(&a, &b);

        assert_eq!(results.len(), 4);
        assert_eq!(results[0].gcd, 4);
        assert_eq!(results[1].gcd, 5);
        assert_eq!(results[2].gcd, 1);
        assert_eq!(results[3].gcd, 25);

        assert!(!results[0].coprime);
        assert!(!results[1].coprime);
        assert!(results[2].coprime);
        assert!(!results[3].coprime);
    }

    #[test]
    fn test_cpu_power_map() {
        // 2^10 = 1024 (unbounded)
        assert_eq!(cpu_power_map(2, 10, None), 1024);
        // x=1 → always 1
        assert_eq!(cpu_power_map(1, 99, None), 1);
        // x=0 → always 0
        assert_eq!(cpu_power_map(0, 5, None), 0);
        // saturation: 2^64 overflows → u64::MAX
        assert_eq!(cpu_power_map(2, 64, None), u64::MAX);
    }

    #[test]
    fn test_cpu_power_map_with_modulus() {
        // 2^10 mod 1000 = 24
        assert_eq!(cpu_power_map(2, 10, Some(1000)), 24);
        // 3^0 mod 7 = 1
        assert_eq!(cpu_power_map(3, 0, Some(7)), 1);
        // 5^1 mod 13 = 5
        assert_eq!(cpu_power_map(5, 1, Some(13)), 5);
        // Result is clamped to 1 when modular result would be 0
        // (x=7, m=0, mod=7 → 1 since 7^0 = 1 ≠ 0, fine)
        assert_eq!(cpu_power_map(7, 0, Some(7)), 1);
    }
}
