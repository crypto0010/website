# FPGA Implementation of DCL Core Models — Design Document

## Overview

**Target:** Digilent Nexys 4 DDR — Xilinx Artix-7 XC7A100T-1CSG324C
**Scope:** Core DCL operations only (GCD, Power Map, Coprimality Check)
**HDL:** Verilog / SystemVerilog
**Toolchain:** Vivado ML Standard (free edition)
**Approach:** Single-unit sequential (Approach 1) — minimal LUT usage

## Board Resources

| Resource | Available | Used (est.) | % Used |
|----------|-----------|-------------|--------|
| 6-input LUTs | 63,400 | ~3,100 | 4.9% |
| Flip-Flops | 126,800 | ~2,500 | 2.0% |
| DSP48E1 | 240 | 12 | 5.0% |
| Block RAM (Kb) | 4,860 | 72 | 1.5% |
| Clock | 100 MHz (onboard) | 100 MHz | — |

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   TOP MODULE (dcl_top)               │
│                                                      │
│  ┌──────────┐    ┌──────────────────────────────┐    │
│  │  UART    │◄──►│      COMMAND DISPATCHER      │    │
│  │  TX/RX   │    │         (FSM)                │    │
│  └──────────┘    └──────┬───────┬───────┬───────┘    │
│                         │       │       │            │
│               ┌─────────▼──┐ ┌──▼─────┐ ┌▼────────┐ │
│               │  GCD_UNIT  │ │POWER   │ │COPRIME  │ │
│               │ (Stein's)  │ │MAP_UNIT│ │CHECKER  │ │
│               │  64-bit    │ │64-bit  │ │(reuses  │ │
│               └────────────┘ └──┬─────┘ │GCD)     │ │
│                                 │       └─────────┘ │
│                          ┌──────▼──────┐             │
│                          │  MULMOD_64  │             │
│                          │ (DSP-based) │             │
│                          └─────────────┘             │
│                                                      │
│  ┌──────────┐    ┌──────────────┐   ┌────────────┐  │
│  │  BRAM    │    │ DEMO CTRL    │   │ 7-SEG /    │  │
│  │ Labels[] │    │ (buttons/sw) │   │ LEDs       │  │
│  │ Edges[]  │    └──────────────┘   └────────────┘  │
│  └──────────┘                                        │
└─────────────────────────────────────────────────────┘
```

## Compute Units

### GCD Unit (`gcd_unit.v`)

- **Algorithm:** Stein's binary GCD (iterative FSM)
- **Interface:** `a[63:0], b[63:0], start` → `result[63:0], done, is_coprime`
- **States:** IDLE → SHIFT → ODD_A → LOOP → DONE
- **Cycles:** 128 max (worst case for 64-bit inputs)
- **Resources:** ~500 LUTs, 0 DSPs
- **Performance:** 1.28 µs per GCD at 100 MHz (781K ops/s)

### Modular Multiplier (`mulmod_64.v`)

- **Algorithm:** Russian peasant binary doubling (modular mode) or DSP cascade (saturating mode)
- **Interface:** `a[63:0], b[63:0], m[63:0], start` → `result[63:0], done`
- **DSP decomposition:** 64-bit operands split into 18-bit chunks, 12 DSP48E1 slices
- **Cycles:** 64 (modular) or 8 (saturating, pipelined DSP)
- **Resources:** ~400 LUTs, 12 DSP48E1

### Power Map Unit (`power_map_unit.v`)

- **Algorithm:** Binary exponentiation reusing `mulmod_64`
- **Interface:** `x[63:0], m[31:0], modulus[63:0], start` → `result[63:0], done`
- **States:** IDLE → CHECK_BIT → MULTIPLY → SQUARE → SHIFT → DONE
- **Cycles:** 2 × mulmod_calls × bits(m) ≈ 4,096 max
- **Resources:** ~800 LUTs, 0 DSPs (reuses mulmod)
- **Zero result mapped to 1** (same convention as CUDA kernel)

### Coprimality Checker (`coprime_checker.v`)

- **Algorithm:** Sequential edge iteration, reuses `gcd_unit`
- **Interface:** `start, num_edges[7:0]` → `all_coprime, done, fail_edge[7:0]`
- **Reads labels[] and edges[] from BRAM**
- **Resources:** ~300 LUTs, 0 DSPs (reuses GCD)
- **Performance:** 128 × num_edges cycles

## Interface

### UART Protocol

- **Physical:** 115200 baud, 8N1, USB-UART via onboard FTDI
- **Format:** `[CMD 1B][LEN 1B][PAYLOAD 0-255B]`

| CMD | Name | Payload In | Payload Out |
|-----|------|------------|-------------|
| 0x01 | GCD | a[8B] b[8B] | gcd[8B] coprime[1B] |
| 0x02 | POWER_MAP | x[8B] m[4B] mod[8B] | result[8B] |
| 0x03 | STORE_LABEL | idx[1B] label[8B] | ACK[1B] |
| 0x04 | STORE_EDGE | idx[1B] u[1B] v[1B] | ACK[1B] |
| 0x05 | CHECK_COPRIME | num_edges[1B] | pass[1B] fail_idx[1B] |
| 0x06 | EVOLVE | m[4B] mod[8B] steps[1B] | labels[n×8B] |
| 0x07 | STATUS | (none) | n_labels[1B] n_edges[1B] |

- **Flow control:** FPGA sends 0xFF (BUSY) if command arrives during processing

### Standalone Demo Mode

- **SW15 = 0:** UART mode, **SW15 = 1:** Demo mode
- **BTNC:** GCD demo (SW[7:0]=a, SW[14:8]=b → 7-seg shows GCD, LD0=coprime)
- **BTNU:** Power Map demo (SW[7:0]=base, SW[11:8]=exp → 7-seg shows result)
- **BTND:** Coprime Check (hardcoded P_5, evolves per press → 7-seg=step, LEDs=edge status)
- **LD15 RGB:** Green=idle, Blue=processing, Red=error

### BRAM Layout

- **Labels:** 1× BRAM18, addresses 0x00–0x1F, 32 labels × 64-bit
- **Edges:** 1× BRAM18, addresses 0x00–0xFF, 256 edges × (u8, u8)
- **Supports:** Graphs up to 32 vertices, 256 edges

## File Structure

```
dcl-fpga/
├── rtl/
│   ├── dcl_top.v            # Top module, clock/reset
│   ├── gcd_unit.v           # Stein's 64-bit GCD
│   ├── mulmod_64.v          # Modular multiplier (DSP)
│   ├── power_map_unit.v     # Binary exponentiation
│   ├── coprime_checker.v    # Edge coprimality FSM
│   ├── uart_rx.v            # UART receiver
│   ├── uart_tx.v            # UART transmitter
│   ├── cmd_dispatch.v       # Command protocol FSM
│   ├── demo_ctrl.v          # Standalone demo controller
│   └── seven_seg.v          # 7-segment display driver
├── tb/
│   ├── tb_gcd_unit.v        # GCD testbench
│   ├── tb_power_map.v       # Power map testbench
│   ├── tb_coprime_checker.v # Coprimality testbench
│   └── tb_dcl_top.v         # Full system testbench
├── constraints/
│   └── nexys4ddr.xdc        # Pin constraints for Nexys 4 DDR
├── scripts/
│   └── build.tcl            # Vivado non-project batch build
├── host/
│   └── dcl_fpga_host/       # Rust crate for UART communication
│       ├── Cargo.toml       # depends on serialport crate
│       └── src/lib.rs       # Host driver: send commands, parse responses
└── docs/
    └── pinout.md            # Pin mapping reference
```

## Verification Strategy

1. **Unit testbenches:** Each RTL module gets a Verilog testbench
2. **Reference vectors:** Generated by Rust `dcl-core` (write test vectors to `.mem` files)
3. **Bit-exact match:** FPGA output must equal `dcl-core` output for all test cases
4. **Coverage targets:**
   - GCD: edge cases (0,1), coprime pairs, non-coprime, large primes, powers of 2
   - Power Map: m=0,1,2,3, large m, modular vs unbounded, overflow saturation
   - Coprime Check: valid P_5 labeling, violated labeling, empty graph

## Design Decisions

1. **Sequential over parallel:** UART I/O (~1.4ms per command) is 1000× slower than compute (~1.3µs). Parallelism would not improve end-to-end latency.
2. **DSP for multiply, not LUT:** 12 DSP48E1 slices replace ~3,000–5,000 LUTs of fabric multiplier logic.
3. **GCD reuse in coprimality checker:** Saves ~500 LUTs by time-sharing the single GCD unit.
4. **32-vertex / 256-edge limit:** Covers all practical DCL research graphs (P_n, C_n, W_n up to n=32, K_n up to n=22, Q_n up to n=4) within 2 BRAM blocks.
5. **Fixed 100 MHz clock:** -1 speed grade comfortably supports 64-bit iterative logic at this frequency. No PLL needed.
