//! GPU-accelerated NIST test data generation from DCL evolution.

use crate::context::CudaContext;
use crate::error::GpuResult;
use crate::kernels::evolve::GpuEvolve;

/// GPU NIST data generator — evolves labels and collects bytes.
pub struct GpuNistGen {
    evolve: GpuEvolve,
}

impl GpuNistGen {
    pub fn new(ctx: CudaContext) -> GpuResult<Self> {
        let evolve = GpuEvolve::new(ctx)?;
        Ok(GpuNistGen { evolve })
    }

    /// Generate NIST test data by evolving labels for `steps` iterations.
    /// Returns raw bytes (each label as 8 LE bytes per step).
    pub fn generate(
        &self,
        initial_labels: &[u64],
        m: u32,
        modulus: u64,
        steps: usize,
    ) -> GpuResult<Vec<u8>> {
        let n = initial_labels.len();
        let mut data = Vec::with_capacity(n * 8 * steps);
        let mut current = initial_labels.to_vec();

        for _ in 0..steps {
            current = self.evolve.evolve_steps(&current, m, modulus, 1)?;
            for &label in &current {
                data.extend_from_slice(&label.to_le_bytes());
            }
        }

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcl_core::labeling::Labeling;
    use dcl_core::transform::PowerMap;

    #[test]
    fn nist_gen_matches_cpu() {
        let ctx = match CudaContext::new() {
            Ok(c) => c,
            Err(_) => { println!("No CUDA device — skipping"); return; }
        };

        let gen = GpuNistGen::new(ctx).unwrap();
        let labels = vec![2, 3, 5, 7, 11];
        let gpu_data = gen.generate(&labels, 2, 0, 10).unwrap();

        // CPU reference
        let pm = PowerMap::new(2);
        let mut cpu_data = Vec::new();
        let mut lab = Labeling::new(labels.clone());
        for _ in 0..10 {
            lab.evolve_in_place(&pm);
            for &l in &lab.labels {
                cpu_data.extend_from_slice(&l.to_le_bytes());
            }
        }

        assert_eq!(gpu_data.len(), cpu_data.len());
        assert_eq!(gpu_data, cpu_data, "GPU NIST data must match CPU");
        println!("NIST gen: {} bytes match CPU", gpu_data.len());
    }
}
