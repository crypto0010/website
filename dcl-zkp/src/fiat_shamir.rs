// dcl-zkp/src/fiat_shamir.rs
//! Fiat-Shamir transform for non-interactive proofs

use sha2::{Sha256, Digest};

/// Transcript for Fiat-Shamir transform
pub struct FiatShamirTranscript {
    hasher: Sha256,
}

impl FiatShamirTranscript {
    pub fn new(label: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(label);
        FiatShamirTranscript { hasher }
    }

    /// Append a message to the transcript
    pub fn append_message(&mut self, label: &[u8], message: &[u8]) {
        self.hasher.update(label);
        self.hasher.update(&(message.len() as u64).to_le_bytes());
        self.hasher.update(message);
    }

    /// Append a u64 value to the transcript
    pub fn append_u64(&mut self, label: &[u8], value: u64) {
        self.append_message(label, &value.to_le_bytes());
    }

    /// Get a challenge value
    pub fn challenge_u64(&mut self, label: &[u8]) -> u64 {
        self.append_message(label, b"challenge");
        let hash = self.hasher.finalize_reset();
        u64::from_le_bytes([
            hash[0], hash[1], hash[2], hash[3],
            hash[4], hash[5], hash[6], hash[7],
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fiat_shamir() {
        let mut transcript = FiatShamirTranscript::new(b"test");

        transcript.append_u64(b"value", 42);
        let challenge = transcript.challenge_u64(b"c");

        assert_ne!(challenge, 0);
    }
}
