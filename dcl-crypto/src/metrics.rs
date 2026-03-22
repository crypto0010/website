// dcl-crypto/src/metrics.rs
//! Sieve performance metrics and reporting.

#[derive(Debug, Default, Clone)]
pub struct SieveMetrics {
    pub attempts: usize,
    pub rejections: usize,
    pub repair_count: u64,
    pub elapsed_ms: u64,
}

impl SieveMetrics {
    pub fn success_rate(&self) -> f64 {
        if self.attempts == 0 {
            return 0.0;
        }
        (self.attempts - self.rejections) as f64 / self.attempts as f64
    }

    pub fn print_report(&self, label: &str) {
        println!("=== Sieve Report: {label} ===");
        println!("  Attempts    : {}", self.attempts);
        println!("  Rejections  : {}", self.rejections);
        println!("  Repairs     : {}", self.repair_count);
        println!("  Success rate: {:.4}%", self.success_rate() * 100.0);
        println!("  Elapsed     : {} ms", self.elapsed_ms);
    }
}
