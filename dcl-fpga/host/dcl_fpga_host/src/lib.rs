//! Host driver for DCL FPGA accelerator.
//! Communicates via UART at 115200 baud using the binary protocol
//! defined in docs/plans/2026-03-14-fpga-artix7-design.md.

use serialport::SerialPort;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum FpgaError {
    #[error("Serial port error: {0}")]
    Serial(#[from] serialport::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("FPGA busy")]
    Busy,
    #[error("Unexpected response length: expected {expected}, got {got}")]
    BadResponse { expected: usize, got: usize },
}

pub type Result<T> = std::result::Result<T, FpgaError>;

/// FPGA command codes
const CMD_GCD: u8 = 0x01;
const CMD_POWER_MAP: u8 = 0x02;
const CMD_STORE_LABEL: u8 = 0x03;
const CMD_STORE_EDGE: u8 = 0x04;
const CMD_CHECK_COPRIME: u8 = 0x05;
const CMD_STATUS: u8 = 0x07;

/// Connection to the DCL FPGA over UART.
pub struct FpgaConnection {
    port: Box<dyn SerialPort>,
}

impl FpgaConnection {
    /// Open a connection to the FPGA.
    /// Typical port: "COM3" on Windows, "/dev/ttyUSB0" on Linux.
    pub fn open(port_name: &str) -> Result<Self> {
        let port = serialport::new(port_name, 115_200)
            .timeout(Duration::from_secs(2))
            .open()?;
        Ok(FpgaConnection { port })
    }

    /// Send a command and receive the response.
    fn command(&mut self, cmd: u8, payload: &[u8], resp_len: usize) -> Result<Vec<u8>> {
        // Send: [CMD][LEN][PAYLOAD...]
        self.port.write_all(&[cmd, payload.len() as u8])?;
        if !payload.is_empty() {
            self.port.write_all(payload)?;
        }
        self.port.flush()?;

        // Read response
        let mut buf = vec![0u8; resp_len];
        self.port.read_exact(&mut buf)?;

        // Check for BUSY (0xFF as first byte)
        if resp_len > 0 && buf[0] == 0xFF {
            return Err(FpgaError::Busy);
        }

        Ok(buf)
    }

    /// Compute GCD of two 64-bit values. Returns (gcd, is_coprime).
    pub fn gcd(&mut self, a: u64, b: u64) -> Result<(u64, bool)> {
        let mut payload = Vec::with_capacity(16);
        payload.extend_from_slice(&a.to_le_bytes());
        payload.extend_from_slice(&b.to_le_bytes());

        let resp = self.command(CMD_GCD, &payload, 9)?;
        let gcd_val = u64::from_le_bytes(resp[0..8].try_into().unwrap());
        let coprime = resp[8] != 0;
        Ok((gcd_val, coprime))
    }

    /// Compute x^m mod modulus (modulus=0 for unbounded). Returns result.
    pub fn power_map(&mut self, x: u64, m: u32, modulus: u64) -> Result<u64> {
        let mut payload = Vec::with_capacity(20);
        payload.extend_from_slice(&x.to_le_bytes());
        payload.extend_from_slice(&m.to_le_bytes());
        payload.extend_from_slice(&modulus.to_le_bytes());

        let resp = self.command(CMD_POWER_MAP, &payload, 8)?;
        Ok(u64::from_le_bytes(resp[0..8].try_into().unwrap()))
    }

    /// Store a label in FPGA BRAM.
    pub fn store_label(&mut self, idx: u8, label: u64) -> Result<()> {
        let mut payload = Vec::with_capacity(9);
        payload.push(idx);
        payload.extend_from_slice(&label.to_le_bytes());
        let _resp = self.command(CMD_STORE_LABEL, &payload, 1)?;
        Ok(())
    }

    /// Store an edge in FPGA BRAM.
    pub fn store_edge(&mut self, idx: u8, u: u8, v: u8) -> Result<()> {
        let payload = vec![idx, u, v];
        let _resp = self.command(CMD_STORE_EDGE, &payload, 1)?;
        Ok(())
    }

    /// Check coprimality of all stored edges. Returns (all_coprime, fail_edge_idx).
    pub fn check_coprime(&mut self, num_edges: u8) -> Result<(bool, u8)> {
        let resp = self.command(CMD_CHECK_COPRIME, &[num_edges], 2)?;
        Ok((resp[0] != 0, resp[1]))
    }

    /// Query FPGA status. Returns (n_labels, n_edges).
    pub fn status(&mut self) -> Result<(u8, u8)> {
        let resp = self.command(CMD_STATUS, &[], 2)?;
        Ok((resp[0], resp[1]))
    }
}
