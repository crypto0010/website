// dcl-security/src/security_levels.rs
//! Adaptive security levels for easy configuration.
//! Users can choose security/performance trade-offs.

use std::time::Duration;

/// Security level configuration for DCL operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    /// Testing mode - fast but insecure (for development only)
    Testing,

    /// Standard security - balanced security/performance
    Standard,

    /// High security - slower but more secure
    High,

    /// Paranoid security - maximum security, very slow
    Paranoid,
}

impl SecurityLevel {
    /// Get proof-of-work iterations for this security level.
    pub fn pow_iterations(&self) -> usize {
        match self {
            SecurityLevel::Testing => 100,
            SecurityLevel::Standard => 10_000,
            SecurityLevel::High => 100_000,
            SecurityLevel::Paranoid => 1_000_000,
        }
    }

    /// Get Miller-Rabin rounds for primality testing.
    pub fn miller_rabin_rounds(&self) -> usize {
        match self {
            SecurityLevel::Testing => 5,
            SecurityLevel::Standard => 40,
            SecurityLevel::High => 64,
            SecurityLevel::Paranoid => 128,
        }
    }

    /// Get number of observation steps for security games.
    pub fn observation_steps(&self) -> usize {
        match self {
            SecurityLevel::Testing => 2,
            SecurityLevel::Standard => 5,
            SecurityLevel::High => 10,
            SecurityLevel::Paranoid => 20,
        }
    }

    /// Get minimum graph size for this security level.
    pub fn min_graph_size(&self) -> usize {
        match self {
            SecurityLevel::Testing => 3,
            SecurityLevel::Standard => 5,
            SecurityLevel::High => 8,
            SecurityLevel::Paranoid => 12,
        }
    }

    /// Get salt size in bytes.
    pub fn salt_size(&self) -> usize {
        match self {
            SecurityLevel::Testing => 16,
            SecurityLevel::Standard => 32,
            SecurityLevel::High => 48,
            SecurityLevel::Paranoid => 64,
        }
    }

    /// Get expected time for this security level.
    pub fn expected_duration(&self) -> Duration {
        match self {
            SecurityLevel::Testing => Duration::from_millis(10),
            SecurityLevel::Standard => Duration::from_millis(100),
            SecurityLevel::High => Duration::from_secs(1),
            SecurityLevel::Paranoid => Duration::from_secs(10),
        }
    }

    /// Get security bits (approximate).
    pub fn security_bits(&self) -> usize {
        match self {
            SecurityLevel::Testing => 32,
            SecurityLevel::Standard => 80,
            SecurityLevel::High => 128,
            SecurityLevel::Paranoid => 256,
        }
    }

    /// Check if configuration meets security level requirements.
    pub fn validate_config(&self, graph_size: usize, mr_rounds: usize) -> Result<(), String> {
        if graph_size < self.min_graph_size() {
            return Err(format!(
                "Graph size {} too small for {:?} security (minimum: {})",
                graph_size,
                self,
                self.min_graph_size()
            ));
        }

        if mr_rounds < self.miller_rabin_rounds() {
            return Err(format!(
                "Miller-Rabin rounds {} insufficient for {:?} security (minimum: {})",
                mr_rounds,
                self,
                self.miller_rabin_rounds()
            ));
        }

        Ok(())
    }

    /// Get recommended configuration summary.
    pub fn summary(&self) -> String {
        format!(
            "{:?} Security Level:\n\
             - PoW Iterations: {}\n\
             - MR Rounds: {}\n\
             - Observation Steps: {}\n\
             - Min Graph Size: {}\n\
             - Security Bits: ~{}\n\
             - Expected Time: {:?}",
            self,
            self.pow_iterations(),
            self.miller_rabin_rounds(),
            self.observation_steps(),
            self.min_graph_size(),
            self.security_bits(),
            self.expected_duration()
        )
    }
}

impl Default for SecurityLevel {
    fn default() -> Self {
        SecurityLevel::Standard
    }
}

impl std::fmt::Display for SecurityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityLevel::Testing => write!(f, "Testing (fast, insecure)"),
            SecurityLevel::Standard => write!(f, "Standard (balanced)"),
            SecurityLevel::High => write!(f, "High (secure, slower)"),
            SecurityLevel::Paranoid => write!(f, "Paranoid (maximum security)"),
        }
    }
}

impl std::str::FromStr for SecurityLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "testing" | "test" | "fast" => Ok(SecurityLevel::Testing),
            "standard" | "normal" | "default" => Ok(SecurityLevel::Standard),
            "high" | "secure" => Ok(SecurityLevel::High),
            "paranoid" | "maximum" | "max" => Ok(SecurityLevel::Paranoid),
            _ => Err(format!("Unknown security level: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_levels_ordered() {
        assert!(SecurityLevel::Testing.pow_iterations() < SecurityLevel::Standard.pow_iterations());
        assert!(SecurityLevel::Standard.pow_iterations() < SecurityLevel::High.pow_iterations());
        assert!(SecurityLevel::High.pow_iterations() < SecurityLevel::Paranoid.pow_iterations());
    }

    #[test]
    fn validation_works() {
        let level = SecurityLevel::High;

        // Should pass
        assert!(level.validate_config(10, 70).is_ok());

        // Should fail - graph too small
        assert!(level.validate_config(5, 70).is_err());

        // Should fail - insufficient MR rounds
        assert!(level.validate_config(10, 20).is_err());
    }

    #[test]
    fn parse_from_string() {
        assert_eq!("standard".parse::<SecurityLevel>().unwrap(), SecurityLevel::Standard);
        assert_eq!("high".parse::<SecurityLevel>().unwrap(), SecurityLevel::High);
        assert_eq!("paranoid".parse::<SecurityLevel>().unwrap(), SecurityLevel::Paranoid);
    }

    #[test]
    fn summary_contains_info() {
        let summary = SecurityLevel::Standard.summary();
        assert!(summary.contains("Standard"));
        assert!(summary.contains("10000")); // PoW iterations
    }
}
