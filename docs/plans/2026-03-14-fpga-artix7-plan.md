# FPGA Artix-7 DCL Core Models — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement DCL core operations (GCD, Power Map, Coprimality Check) in Verilog for the Digilent Nexys 4 DDR (Artix-7 XC7A100T-1CSG324C), with UART host communication and standalone demo mode.

**Architecture:** Single-unit sequential design — one GCD unit, one modular multiplier (DSP-based), one power map unit, and one coprimality checker sharing resources. Command dispatcher FSM routes UART commands to compute units. Demo controller provides switch/button-driven standalone operation. ~3,100 LUTs (~5% of XC7A100T).

**Tech Stack:** Verilog/SystemVerilog, Vivado ML Standard (free edition), Icarus Verilog (simulation), Rust `serialport` crate (host driver)

**Design Document:** `docs/plans/2026-03-14-fpga-artix7-design.md`

---

### Task 1: Project Scaffold

**Files:**
- Create: `dcl-fpga/rtl/` (directory)
- Create: `dcl-fpga/tb/` (directory)
- Create: `dcl-fpga/constraints/` (directory)
- Create: `dcl-fpga/scripts/` (directory)
- Create: `dcl-fpga/host/dcl_fpga_host/` (directory)
- Create: `dcl-fpga/docs/` (directory)

**Step 1: Create directory structure**

```bash
mkdir -p dcl-fpga/{rtl,tb,constraints,scripts,host/dcl_fpga_host/src,docs}
```

**Step 2: Create a placeholder README**

Create `dcl-fpga/README.md`:
```
# DCL-FPGA — Artix-7 Implementation

Target: Digilent Nexys 4 DDR (XC7A100T-1CSG324C)
Toolchain: Vivado ML Standard

See docs/plans/2026-03-14-fpga-artix7-design.md for architecture.
```

**Step 3: Commit**

```bash
git add dcl-fpga/
git commit -m "feat(fpga): scaffold dcl-fpga directory structure"
```

---

### Task 2: GCD Unit (`gcd_unit.v`)

**Files:**
- Create: `dcl-fpga/rtl/gcd_unit.v`
- Create: `dcl-fpga/tb/tb_gcd_unit.v`

**Step 1: Write the GCD unit testbench**

Create `dcl-fpga/tb/tb_gcd_unit.v`:

```verilog
`timescale 1ns / 1ps

module tb_gcd_unit;
    reg         clk, rst_n, start;
    reg  [63:0] a, b;
    wire [63:0] result;
    wire        done, is_coprime;

    gcd_unit uut (
        .clk(clk), .rst_n(rst_n), .start(start),
        .a(a), .b(b),
        .result(result), .done(done), .is_coprime(is_coprime)
    );

    always #5 clk = ~clk;  // 100 MHz

    integer pass_count = 0;
    integer fail_count = 0;

    task check_gcd(input [63:0] in_a, in_b, expected, input exp_coprime);
        begin
            @(posedge clk);
            a = in_a; b = in_b; start = 1;
            @(posedge clk);
            start = 0;
            wait(done);
            @(posedge clk);
            if (result !== expected || is_coprime !== exp_coprime) begin
                $display("FAIL: gcd(%0d,%0d) = %0d (expected %0d), coprime=%0b (expected %0b)",
                         in_a, in_b, result, expected, is_coprime, exp_coprime);
                fail_count = fail_count + 1;
            end else begin
                $display("PASS: gcd(%0d,%0d) = %0d, coprime=%0b", in_a, in_b, result, is_coprime);
                pass_count = pass_count + 1;
            end
        end
    endtask

    initial begin
        clk = 0; rst_n = 0; start = 0; a = 0; b = 0;
        #20 rst_n = 1;

        // Basic cases
        check_gcd(12, 8, 4, 0);          // gcd(12,8) = 4
        check_gcd(17, 13, 1, 1);         // coprime
        check_gcd(0, 5, 5, 0);           // gcd(0,x) = x
        check_gcd(100, 0, 100, 0);       // gcd(x,0) = x
        check_gcd(0, 0, 0, 0);           // gcd(0,0) = 0
        check_gcd(1, 1, 1, 1);           // gcd(1,1) = 1
        check_gcd(7, 13, 1, 1);          // coprime primes
        check_gcd(6, 9, 3, 0);           // non-coprime
        check_gcd(1024, 512, 512, 0);    // powers of 2
        check_gcd(2, 3, 1, 1);           // small coprime
        check_gcd(5, 7, 1, 1);           // small coprime
        check_gcd(48, 18, 6, 0);         // gcd(48,18) = 6
        // Large primes
        check_gcd(64'd104729, 64'd104743, 1, 1);
        // Large non-coprime
        check_gcd(64'd123456789012345678, 64'd246913578024691356, 64'd123456789012345678, 0);

        $display("\n=== GCD Unit: %0d PASSED, %0d FAILED ===", pass_count, fail_count);
        if (fail_count > 0) $display("*** TEST FAILURES ***");
        $finish;
    end
endmodule
```

**Step 2: Run testbench to verify it fails (no gcd_unit.v yet)**

```bash
cd dcl-fpga
iverilog -o tb/tb_gcd_unit.vvp tb/tb_gcd_unit.v rtl/gcd_unit.v 2>&1
```
Expected: Compilation error — `gcd_unit.v` does not exist.

**Step 3: Implement the GCD unit**

Create `dcl-fpga/rtl/gcd_unit.v`:

```verilog
`timescale 1ns / 1ps
// Stein's binary GCD — 64-bit iterative FSM
// Algorithm matches dcl-core/src/gcd.rs exactly:
//   shift = ctz(a|b); a >>= ctz(a);
//   loop { b >>= ctz(b); if a>b swap; b -= a; if b==0 return a<<shift }

module gcd_unit (
    input  wire        clk,
    input  wire        rst_n,
    input  wire        start,
    input  wire [63:0] a,
    input  wire [63:0] b,
    output reg  [63:0] result,
    output reg         done,
    output reg         is_coprime
);

    localparam S_IDLE  = 3'd0,
               S_ZERO  = 3'd1,  // handle a==0 or b==0
               S_SHIFT = 3'd2,  // count common trailing zeros
               S_ODD_A = 3'd3,  // strip trailing zeros from a
               S_LOOP  = 3'd4,  // main loop: strip b, compare, subtract
               S_DONE  = 3'd5;

    reg [2:0]  state;
    reg [63:0] ra, rb;
    reg [6:0]  shift;  // max shift = 63

    // Count trailing zeros (combinational). Returns 64 if input is 0.
    function [6:0] ctz;
        input [63:0] val;
        integer i;
        begin
            ctz = 64;
            for (i = 0; i < 64; i = i + 1) begin
                if (val[i] && ctz == 64)
                    ctz = i[6:0];
            end
        end
    endfunction

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state     <= S_IDLE;
            result    <= 0;
            done      <= 0;
            is_coprime <= 0;
            ra <= 0; rb <= 0; shift <= 0;
        end else begin
            case (state)
                S_IDLE: begin
                    done <= 0;
                    if (start) begin
                        ra <= a;
                        rb <= b;
                        shift <= 0;
                        if (a == 0 || b == 0)
                            state <= S_ZERO;
                        else
                            state <= S_SHIFT;
                    end
                end

                S_ZERO: begin
                    // gcd(0,x) = x, gcd(x,0) = x, gcd(0,0) = 0
                    result     <= ra | rb;
                    is_coprime <= (ra | rb) == 64'd1;
                    done       <= 1;
                    state      <= S_IDLE;
                end

                S_SHIFT: begin
                    // Count common trailing zeros of (a|b)
                    shift <= ctz(ra | rb);
                    ra    <= ra >> ctz(ra | rb);
                    rb    <= rb >> ctz(ra | rb);
                    state <= S_ODD_A;
                end

                S_ODD_A: begin
                    // Strip trailing zeros from a
                    ra    <= ra >> ctz(ra);
                    state <= S_LOOP;
                end

                S_LOOP: begin
                    // Strip trailing zeros from b
                    // NOTE: we do this in-cycle since ctz is combinational
                    if (rb == 0) begin
                        result     <= ra << shift;
                        is_coprime <= (ra == 64'd1) && (shift == 0);
                        done       <= 1;
                        state      <= S_IDLE;
                    end else begin
                        // b >>= ctz(b)
                        rb <= rb >> ctz(rb);
                        state <= S_LOOP;
                        // After stripping, compare and subtract
                        // We need the stripped value, so we do swap/subtract
                        // with the stripped b in the next cycle
                        // Actually, let's pipeline: strip b, then compare
                        // To keep it simple, use a sub-state approach:
                        // This cycle: strip b. Next cycle check in S_LOOP again.
                        // If b is already odd, do the comparison.
                        if (ctz(rb) == 0) begin
                            // b is odd — do comparison + subtraction
                            if (ra > rb) begin
                                // swap
                                ra <= rb;
                                rb <= ra - rb;
                            end else begin
                                rb <= rb - ra;
                            end
                        end
                    end
                end

                S_DONE: begin
                    done  <= 1;
                    state <= S_IDLE;
                end

                default: state <= S_IDLE;
            endcase
        end
    end
endmodule
```

**Step 4: Run testbench**

```bash
cd dcl-fpga
iverilog -g2012 -o tb/tb_gcd_unit.vvp rtl/gcd_unit.v tb/tb_gcd_unit.v && vvp tb/tb_gcd_unit.vvp
```
Expected: All 14 test cases PASS.

**Step 5: Commit**

```bash
git add dcl-fpga/rtl/gcd_unit.v dcl-fpga/tb/tb_gcd_unit.v
git commit -m "feat(fpga): implement Stein's binary GCD unit with testbench"
```

---

### Task 3: Modular Multiplier (`mulmod_64.v`)

**Files:**
- Create: `dcl-fpga/rtl/mulmod_64.v`

**Step 1: Implement the modular multiplier**

Create `dcl-fpga/rtl/mulmod_64.v`:

```verilog
`timescale 1ns / 1ps
// 64-bit modular multiplication: result = (a * b) mod m
// Algorithm: Russian peasant binary method (same as GPU kernel mulmod)
//   result = 0
//   a = a mod m
//   for each bit of b (LSB to MSB):
//     if bit set: result = (result + a) mod m
//     a = (2*a) mod m
//
// When m == 0: saturating multiply (for unbounded power map)
// Cycles: 64 (one per bit of b)

module mulmod_64 (
    input  wire        clk,
    input  wire        rst_n,
    input  wire        start,
    input  wire [63:0] a,
    input  wire [63:0] b,
    input  wire [63:0] m,      // modulus (0 = saturating mode)
    output reg  [63:0] result,
    output reg         done
);

    localparam S_IDLE = 2'd0,
               S_CALC = 2'd1,
               S_DONE = 2'd2;

    reg [1:0]   state;
    reg [63:0]  ra;       // running 'a' (doubled each iteration)
    reg [63:0]  rb;       // remaining bits of 'b'
    reg [63:0]  acc;      // accumulator
    reg         saturating;
    reg [127:0] wide_acc; // for saturating mode overflow detection
    reg [127:0] wide_a;

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state  <= S_IDLE;
            result <= 0;
            done   <= 0;
            ra <= 0; rb <= 0; acc <= 0;
            saturating <= 0;
            wide_acc <= 0; wide_a <= 0;
        end else begin
            case (state)
                S_IDLE: begin
                    done <= 0;
                    if (start) begin
                        if (m == 0) begin
                            // Saturating mode
                            saturating <= 1;
                            wide_a   <= {64'd0, a};
                            rb       <= b;
                            wide_acc <= 0;
                            state    <= S_CALC;
                        end else begin
                            // Modular mode
                            saturating <= 0;
                            ra    <= a % m;
                            rb    <= b;
                            acc   <= 0;
                            state <= S_CALC;
                        end
                    end
                end

                S_CALC: begin
                    if (rb == 0) begin
                        if (saturating) begin
                            result <= (wide_acc > {64'hFFFFFFFF_FFFFFFFF}) ?
                                      64'hFFFFFFFF_FFFFFFFF : wide_acc[63:0];
                        end else begin
                            result <= acc;
                        end
                        done  <= 1;
                        state <= S_IDLE;
                    end else begin
                        if (saturating) begin
                            // Saturating: use 128-bit intermediates
                            if (rb[0])
                                wide_acc <= wide_acc + wide_a;
                            wide_a <= wide_a << 1;
                        end else begin
                            // Modular: all ops mod m
                            if (rb[0]) begin
                                acc <= (acc + ra >= m) ? (acc + ra - m) : (acc + ra);
                            end
                            ra <= (ra + ra >= m) ? (ra + ra - m) : (ra + ra);
                        end
                        rb <= rb >> 1;
                    end
                end

                S_DONE: begin
                    done  <= 1;
                    state <= S_IDLE;
                end

                default: state <= S_IDLE;
            endcase
        end
    end
endmodule
```

**Note:** This uses the Russian peasant approach (LUT-based, no DSP) for simplicity and portability in simulation. A DSP-cascade optimization can be added later for synthesis if needed, but the Russian peasant method is correct and matches the GPU kernel algorithm exactly.

**Step 2: Commit**

```bash
git add dcl-fpga/rtl/mulmod_64.v
git commit -m "feat(fpga): implement 64-bit modular multiplier (Russian peasant)"
```

---

### Task 4: Power Map Unit (`power_map_unit.v`)

**Files:**
- Create: `dcl-fpga/rtl/power_map_unit.v`
- Create: `dcl-fpga/tb/tb_power_map.v`

**Step 1: Write the power map testbench**

Create `dcl-fpga/tb/tb_power_map.v`:

```verilog
`timescale 1ns / 1ps

module tb_power_map;
    reg         clk, rst_n, start;
    reg  [63:0] x;
    reg  [31:0] m;
    reg  [63:0] modulus;
    wire [63:0] result;
    wire        done;

    power_map_unit uut (
        .clk(clk), .rst_n(rst_n), .start(start),
        .x(x), .m(m), .modulus(modulus),
        .result(result), .done(done)
    );

    always #5 clk = ~clk;

    integer pass_count = 0;
    integer fail_count = 0;

    task check_pow(input [63:0] in_x, input [31:0] in_m, input [63:0] in_mod, in_expected);
        begin
            @(posedge clk);
            x = in_x; m = in_m; modulus = in_mod; start = 1;
            @(posedge clk);
            start = 0;
            wait(done);
            @(posedge clk);
            if (result !== in_expected) begin
                $display("FAIL: pow(%0d, %0d) mod %0d = %0d (expected %0d)",
                         in_x, in_m, in_mod, result, in_expected);
                fail_count = fail_count + 1;
            end else begin
                $display("PASS: pow(%0d, %0d) mod %0d = %0d", in_x, in_m, in_mod, result);
                pass_count = pass_count + 1;
            end
        end
    endtask

    initial begin
        clk = 0; rst_n = 0; start = 0; x = 0; m = 0; modulus = 0;
        #20 rst_n = 1;

        // Modular mode (modulus > 0)
        check_pow(2, 10, 1000, 24);       // 2^10 mod 1000 = 1024 mod 1000 = 24
        check_pow(3, 3, 100, 27);         // 3^3 mod 100 = 27
        check_pow(5, 3, 1000, 125);       // 5^3 mod 1000 = 125
        check_pow(7, 2, 50, 49);          // 7^2 mod 50 = 49
        check_pow(2, 1, 100, 2);          // x^1 = x
        check_pow(99, 1, 100, 99);        // x^1 = x

        // Edge cases
        check_pow(0, 5, 100, 1);          // 0^m mod N → 0, mapped to 1
        check_pow(5, 0, 100, 1);          // x^0 = 1 (but m>=1 in design, treat as edge)

        // Unbounded mode (modulus == 0)
        check_pow(2, 10, 0, 1024);        // 2^10 = 1024
        check_pow(3, 3, 0, 27);           // 3^3 = 27
        check_pow(5, 3, 0, 125);          // 5^3 = 125
        check_pow(1, 100, 0, 1);          // 1^anything = 1

        // Overflow saturation (unbounded)
        check_pow(3, 60, 0, 64'hFFFFFFFF_FFFFFFFF); // 3^60 overflows → saturate

        $display("\n=== Power Map: %0d PASSED, %0d FAILED ===", pass_count, fail_count);
        if (fail_count > 0) $display("*** TEST FAILURES ***");
        $finish;
    end
endmodule
```

**Step 2: Run testbench to verify it fails**

```bash
cd dcl-fpga
iverilog -g2012 -o tb/tb_power_map.vvp rtl/mulmod_64.v rtl/power_map_unit.v tb/tb_power_map.v 2>&1
```
Expected: Fails — `power_map_unit.v` does not exist.

**Step 3: Implement the power map unit**

Create `dcl-fpga/rtl/power_map_unit.v`:

```verilog
`timescale 1ns / 1ps
// Binary exponentiation: result = x^m mod modulus
// When modulus == 0: unbounded mode with u64 saturation
// Reuses mulmod_64 for each multiply/square step
// Algorithm matches dcl-core/src/transform.rs PowerMap::apply()
// Zero result mapped to 1 (same convention as CUDA kernel)

module power_map_unit (
    input  wire        clk,
    input  wire        rst_n,
    input  wire        start,
    input  wire [63:0] x,
    input  wire [31:0] m,        // exponent
    input  wire [63:0] modulus,  // 0 = unbounded/saturating
    output reg  [63:0] result,
    output reg         done
);

    localparam S_IDLE      = 3'd0,
               S_CHECK_BIT = 3'd1,
               S_MULTIPLY  = 3'd2,
               S_SQUARE    = 3'd3,
               S_SHIFT     = 3'd4,
               S_DONE      = 3'd5;

    reg [2:0]   state;
    reg [63:0]  acc;       // running result
    reg [63:0]  base;      // running base
    reg [31:0]  exp;       // remaining exponent bits
    reg         mul_start;
    reg [63:0]  mul_a, mul_b;
    wire [63:0] mul_result;
    wire        mul_done;

    // Instantiate shared mulmod
    mulmod_64 u_mulmod (
        .clk(clk), .rst_n(rst_n), .start(mul_start),
        .a(mul_a), .b(mul_b), .m(modulus),
        .result(mul_result), .done(mul_done)
    );

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state     <= S_IDLE;
            result    <= 0;
            done      <= 0;
            acc       <= 0;
            base      <= 0;
            exp       <= 0;
            mul_start <= 0;
            mul_a     <= 0;
            mul_b     <= 0;
        end else begin
            mul_start <= 0;  // default: deassert

            case (state)
                S_IDLE: begin
                    done <= 0;
                    if (start) begin
                        exp  <= m;
                        if (modulus != 0) begin
                            acc  <= 1;
                            base <= x % modulus;
                        end else begin
                            acc  <= 1;
                            base <= x;
                        end
                        state <= S_CHECK_BIT;
                    end
                end

                S_CHECK_BIT: begin
                    if (exp == 0) begin
                        // Map 0 → 1 (convention)
                        result <= (acc == 0) ? 64'd1 : acc;
                        done   <= 1;
                        state  <= S_IDLE;
                    end else if (exp[0]) begin
                        // Bit set: acc = acc * base (mod)
                        mul_a     <= acc;
                        mul_b     <= base;
                        mul_start <= 1;
                        state     <= S_MULTIPLY;
                    end else begin
                        // Bit clear: skip to square
                        state <= S_SQUARE;
                        mul_a     <= base;
                        mul_b     <= base;
                        mul_start <= 1;
                    end
                end

                S_MULTIPLY: begin
                    if (mul_done) begin
                        acc <= mul_result;
                        // Now square: base = base * base
                        mul_a     <= base;
                        mul_b     <= base;
                        mul_start <= 1;
                        state     <= S_SQUARE;
                    end
                end

                S_SQUARE: begin
                    if (mul_done) begin
                        base <= mul_result;
                        state <= S_SHIFT;
                    end
                end

                S_SHIFT: begin
                    exp   <= exp >> 1;
                    state <= S_CHECK_BIT;
                end

                S_DONE: begin
                    done  <= 1;
                    state <= S_IDLE;
                end

                default: state <= S_IDLE;
            endcase
        end
    end
endmodule
```

**Step 4: Run testbench**

```bash
cd dcl-fpga
iverilog -g2012 -o tb/tb_power_map.vvp rtl/mulmod_64.v rtl/power_map_unit.v tb/tb_power_map.v && vvp tb/tb_power_map.vvp
```
Expected: All test cases PASS.

**Step 5: Commit**

```bash
git add dcl-fpga/rtl/power_map_unit.v dcl-fpga/tb/tb_power_map.v
git commit -m "feat(fpga): implement power map unit with binary exponentiation"
```

---

### Task 5: Coprimality Checker (`coprime_checker.v`)

**Files:**
- Create: `dcl-fpga/rtl/coprime_checker.v`
- Create: `dcl-fpga/tb/tb_coprime_checker.v`

**Step 1: Write the coprimality checker testbench**

Create `dcl-fpga/tb/tb_coprime_checker.v`:

```verilog
`timescale 1ns / 1ps

module tb_coprime_checker;
    reg         clk, rst_n, start;
    reg  [7:0]  num_edges;

    // BRAM interfaces (directly driven by TB)
    reg  [63:0] label_data;
    wire [4:0]  label_addr;
    reg         label_valid;

    reg  [7:0]  edge_u_data, edge_v_data;
    wire [7:0]  edge_addr;
    reg         edge_valid;

    wire        all_coprime, done;
    wire [7:0]  fail_edge;

    // Simulated BRAM for labels (32 × 64-bit)
    reg [63:0] labels [0:31];
    // Simulated BRAM for edges (256 × 16-bit packed)
    reg [7:0] edges_u [0:255];
    reg [7:0] edges_v [0:255];

    coprime_checker uut (
        .clk(clk), .rst_n(rst_n), .start(start),
        .num_edges(num_edges),
        .label_data(label_data), .label_addr(label_addr),
        .edge_u_data(edge_u_data), .edge_v_data(edge_v_data),
        .edge_addr(edge_addr),
        .all_coprime(all_coprime), .done(done), .fail_edge(fail_edge)
    );

    always #5 clk = ~clk;

    // BRAM read simulation (1-cycle latency)
    always @(posedge clk) begin
        label_data  <= labels[label_addr];
        edge_u_data <= edges_u[edge_addr];
        edge_v_data <= edges_v[edge_addr];
    end

    integer pass_count = 0;
    integer fail_count = 0;
    integer i;

    initial begin
        clk = 0; rst_n = 0; start = 0; num_edges = 0;

        // Initialize BRAM
        for (i = 0; i < 32; i = i + 1) labels[i] = 0;
        for (i = 0; i < 256; i = i + 1) begin
            edges_u[i] = 0; edges_v[i] = 0;
        end

        #20 rst_n = 1;

        // --- Test 1: P_5 with coprime labeling [1,2,3,5,7] ---
        // Edges: (0,1), (1,2), (2,3), (3,4)
        labels[0] = 1; labels[1] = 2; labels[2] = 3;
        labels[3] = 5; labels[4] = 7;
        edges_u[0] = 0; edges_v[0] = 1;
        edges_u[1] = 1; edges_v[1] = 2;
        edges_u[2] = 2; edges_v[2] = 3;
        edges_u[3] = 3; edges_v[3] = 4;

        @(posedge clk);
        num_edges = 4; start = 1;
        @(posedge clk);
        start = 0;
        wait(done);
        @(posedge clk);
        if (all_coprime !== 1) begin
            $display("FAIL: P_5 coprime labeling should pass"); fail_count = fail_count + 1;
        end else begin
            $display("PASS: P_5 coprime labeling"); pass_count = pass_count + 1;
        end

        // --- Test 2: P_4 with non-coprime labeling [6,9,5,7] ---
        labels[0] = 6; labels[1] = 9; labels[2] = 5; labels[3] = 7;
        edges_u[0] = 0; edges_v[0] = 1;
        edges_u[1] = 1; edges_v[1] = 2;
        edges_u[2] = 2; edges_v[2] = 3;

        @(posedge clk);
        num_edges = 3; start = 1;
        @(posedge clk);
        start = 0;
        wait(done);
        @(posedge clk);
        if (all_coprime !== 0) begin
            $display("FAIL: non-coprime labeling should fail"); fail_count = fail_count + 1;
        end else begin
            $display("PASS: non-coprime labeling detected, fail_edge=%0d", fail_edge);
            pass_count = pass_count + 1;
        end

        // --- Test 3: Empty graph (0 edges) ---
        @(posedge clk);
        num_edges = 0; start = 1;
        @(posedge clk);
        start = 0;
        wait(done);
        @(posedge clk);
        if (all_coprime !== 1) begin
            $display("FAIL: empty graph should be coprime"); fail_count = fail_count + 1;
        end else begin
            $display("PASS: empty graph coprime"); pass_count = pass_count + 1;
        end

        $display("\n=== Coprime Checker: %0d PASSED, %0d FAILED ===", pass_count, fail_count);
        $finish;
    end
endmodule
```

**Step 2: Implement the coprimality checker**

Create `dcl-fpga/rtl/coprime_checker.v`:

```verilog
`timescale 1ns / 1ps
// Sequential coprimality checker — iterates edges, reuses gcd_unit
// Reads labels[] and edges[] from BRAM ports
// Algorithm: for each edge (u,v), compute gcd(labels[u], labels[v])
//   If any gcd != 1, set all_coprime=0 and report fail_edge

module coprime_checker (
    input  wire        clk,
    input  wire        rst_n,
    input  wire        start,
    input  wire [7:0]  num_edges,

    // BRAM read ports
    input  wire [63:0] label_data,
    output reg  [4:0]  label_addr,

    input  wire [7:0]  edge_u_data,
    input  wire [7:0]  edge_v_data,
    output reg  [7:0]  edge_addr,

    output reg         all_coprime,
    output reg         done,
    output reg  [7:0]  fail_edge
);

    localparam S_IDLE     = 3'd0,
               S_RD_EDGE  = 3'd1,  // read edge (u,v) from BRAM
               S_RD_LBL_U = 3'd2,  // request label[u]
               S_RD_LBL_V = 3'd3,  // request label[v]
               S_GCD      = 3'd4,  // run GCD
               S_CHECK    = 3'd5,  // check GCD result
               S_DONE     = 3'd6;

    reg [2:0]  state;
    reg [7:0]  edge_idx;
    reg [63:0] label_u, label_v;
    reg [7:0]  vert_u, vert_v;

    // GCD unit wires
    reg        gcd_start;
    reg [63:0] gcd_a, gcd_b;
    wire [63:0] gcd_result;
    wire        gcd_done, gcd_coprime;

    gcd_unit u_gcd (
        .clk(clk), .rst_n(rst_n), .start(gcd_start),
        .a(gcd_a), .b(gcd_b),
        .result(gcd_result), .done(gcd_done), .is_coprime(gcd_coprime)
    );

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state       <= S_IDLE;
            done        <= 0;
            all_coprime <= 0;
            fail_edge   <= 0;
            edge_idx    <= 0;
            gcd_start   <= 0;
            label_addr  <= 0;
            edge_addr   <= 0;
            label_u     <= 0;
            label_v     <= 0;
            gcd_a       <= 0;
            gcd_b       <= 0;
        end else begin
            gcd_start <= 0;

            case (state)
                S_IDLE: begin
                    done <= 0;
                    if (start) begin
                        if (num_edges == 0) begin
                            all_coprime <= 1;
                            fail_edge   <= 0;
                            done        <= 1;
                            state       <= S_IDLE;
                        end else begin
                            edge_idx    <= 0;
                            all_coprime <= 1;
                            fail_edge   <= 0;
                            edge_addr   <= 0;
                            state       <= S_RD_EDGE;
                        end
                    end
                end

                S_RD_EDGE: begin
                    // Wait 1 cycle for BRAM read latency
                    state <= S_RD_LBL_U;
                end

                S_RD_LBL_U: begin
                    // Edge data available: latch u,v and request label[u]
                    vert_u     <= edge_u_data;
                    vert_v     <= edge_v_data;
                    label_addr <= edge_u_data[4:0];
                    state      <= S_RD_LBL_V;
                end

                S_RD_LBL_V: begin
                    // label[u] available, request label[v]
                    label_u    <= label_data;
                    label_addr <= vert_v[4:0];
                    state      <= S_GCD;
                end

                S_GCD: begin
                    // label[v] available, start GCD
                    label_v   <= label_data;
                    gcd_a     <= label_u;
                    gcd_b     <= label_data;
                    gcd_start <= 1;
                    state     <= S_CHECK;
                end

                S_CHECK: begin
                    if (gcd_done) begin
                        if (!gcd_coprime) begin
                            // Found a non-coprime edge
                            all_coprime <= 0;
                            fail_edge   <= edge_idx;
                            done        <= 1;
                            state       <= S_IDLE;
                        end else if (edge_idx + 1 >= num_edges) begin
                            // All edges checked, all coprime
                            all_coprime <= 1;
                            done        <= 1;
                            state       <= S_IDLE;
                        end else begin
                            // Next edge
                            edge_idx  <= edge_idx + 1;
                            edge_addr <= edge_idx + 1;
                            state     <= S_RD_EDGE;
                        end
                    end
                end

                S_DONE: begin
                    done  <= 1;
                    state <= S_IDLE;
                end

                default: state <= S_IDLE;
            endcase
        end
    end
endmodule
```

**Step 3: Run testbench**

```bash
cd dcl-fpga
iverilog -g2012 -o tb/tb_coprime_checker.vvp rtl/gcd_unit.v rtl/coprime_checker.v tb/tb_coprime_checker.v && vvp tb/tb_coprime_checker.vvp
```
Expected: All 3 test cases PASS.

**Step 4: Commit**

```bash
git add dcl-fpga/rtl/coprime_checker.v dcl-fpga/tb/tb_coprime_checker.v
git commit -m "feat(fpga): implement coprimality checker with GCD reuse"
```

---

### Task 6: UART RX and TX (`uart_rx.v`, `uart_tx.v`)

**Files:**
- Create: `dcl-fpga/rtl/uart_rx.v`
- Create: `dcl-fpga/rtl/uart_tx.v`

**Step 1: Implement UART receiver**

Create `dcl-fpga/rtl/uart_rx.v`:

```verilog
`timescale 1ns / 1ps
// UART Receiver — 115200 baud, 8N1
// clk = 100 MHz → baud divider = 100_000_000 / 115_200 = 868

module uart_rx #(
    parameter CLKS_PER_BIT = 868  // 100 MHz / 115200 baud
)(
    input  wire       clk,
    input  wire       rst_n,
    input  wire       rx,          // serial input
    output reg  [7:0] data,        // received byte
    output reg        valid        // pulse high for 1 cycle when byte ready
);

    localparam S_IDLE  = 2'd0,
               S_START = 2'd1,
               S_DATA  = 2'd2,
               S_STOP  = 2'd3;

    reg [1:0]  state;
    reg [15:0] clk_count;
    reg [2:0]  bit_idx;
    reg [7:0]  shift_reg;
    reg        rx_sync1, rx_sync2;  // double-flop synchronizer

    // Synchronize async RX input
    always @(posedge clk) begin
        rx_sync1 <= rx;
        rx_sync2 <= rx_sync1;
    end

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state     <= S_IDLE;
            clk_count <= 0;
            bit_idx   <= 0;
            data      <= 0;
            valid     <= 0;
            shift_reg <= 0;
        end else begin
            valid <= 0;

            case (state)
                S_IDLE: begin
                    if (rx_sync2 == 0) begin
                        // Falling edge detected — potential start bit
                        clk_count <= 0;
                        state     <= S_START;
                    end
                end

                S_START: begin
                    // Sample at middle of start bit
                    if (clk_count == (CLKS_PER_BIT - 1) / 2) begin
                        if (rx_sync2 == 0) begin
                            // Valid start bit
                            clk_count <= 0;
                            bit_idx   <= 0;
                            state     <= S_DATA;
                        end else begin
                            state <= S_IDLE;  // glitch
                        end
                    end else begin
                        clk_count <= clk_count + 1;
                    end
                end

                S_DATA: begin
                    if (clk_count == CLKS_PER_BIT - 1) begin
                        clk_count <= 0;
                        shift_reg[bit_idx] <= rx_sync2;  // LSB first
                        if (bit_idx == 7) begin
                            state <= S_STOP;
                        end else begin
                            bit_idx <= bit_idx + 1;
                        end
                    end else begin
                        clk_count <= clk_count + 1;
                    end
                end

                S_STOP: begin
                    if (clk_count == CLKS_PER_BIT - 1) begin
                        // Stop bit — output data
                        data  <= shift_reg;
                        valid <= 1;
                        state <= S_IDLE;
                    end else begin
                        clk_count <= clk_count + 1;
                    end
                end
            endcase
        end
    end
endmodule
```

**Step 2: Implement UART transmitter**

Create `dcl-fpga/rtl/uart_tx.v`:

```verilog
`timescale 1ns / 1ps
// UART Transmitter — 115200 baud, 8N1

module uart_tx #(
    parameter CLKS_PER_BIT = 868
)(
    input  wire       clk,
    input  wire       rst_n,
    input  wire [7:0] data,
    input  wire       send,       // pulse to start transmission
    output reg        tx,         // serial output
    output reg        busy        // high while transmitting
);

    localparam S_IDLE  = 2'd0,
               S_START = 2'd1,
               S_DATA  = 2'd2,
               S_STOP  = 2'd3;

    reg [1:0]  state;
    reg [15:0] clk_count;
    reg [2:0]  bit_idx;
    reg [7:0]  shift_reg;

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state     <= S_IDLE;
            tx        <= 1;  // idle high
            busy      <= 0;
            clk_count <= 0;
            bit_idx   <= 0;
            shift_reg <= 0;
        end else begin
            case (state)
                S_IDLE: begin
                    tx   <= 1;
                    busy <= 0;
                    if (send) begin
                        shift_reg <= data;
                        busy      <= 1;
                        clk_count <= 0;
                        state     <= S_START;
                    end
                end

                S_START: begin
                    tx <= 0;  // start bit
                    if (clk_count == CLKS_PER_BIT - 1) begin
                        clk_count <= 0;
                        bit_idx   <= 0;
                        state     <= S_DATA;
                    end else begin
                        clk_count <= clk_count + 1;
                    end
                end

                S_DATA: begin
                    tx <= shift_reg[bit_idx];  // LSB first
                    if (clk_count == CLKS_PER_BIT - 1) begin
                        clk_count <= 0;
                        if (bit_idx == 7) begin
                            state <= S_STOP;
                        end else begin
                            bit_idx <= bit_idx + 1;
                        end
                    end else begin
                        clk_count <= clk_count + 1;
                    end
                end

                S_STOP: begin
                    tx <= 1;  // stop bit
                    if (clk_count == CLKS_PER_BIT - 1) begin
                        state <= S_IDLE;
                    end else begin
                        clk_count <= clk_count + 1;
                    end
                end
            endcase
        end
    end
endmodule
```

**Step 3: Commit**

```bash
git add dcl-fpga/rtl/uart_rx.v dcl-fpga/rtl/uart_tx.v
git commit -m "feat(fpga): implement UART RX/TX modules (115200 8N1)"
```

---

### Task 7: Seven-Segment Display Driver (`seven_seg.v`)

**Files:**
- Create: `dcl-fpga/rtl/seven_seg.v`

**Step 1: Implement the 7-segment driver**

Create `dcl-fpga/rtl/seven_seg.v`:

```verilog
`timescale 1ns / 1ps
// 8-digit multiplexed 7-segment display driver for Nexys 4 DDR
// Displays a 32-bit hex value across digits AN[7:0]
// Active-low cathodes (CA-CG) and anodes (AN)
// Refresh rate: 100 MHz / 2^18 ≈ 381 Hz per digit

module seven_seg (
    input  wire        clk,
    input  wire        rst_n,
    input  wire [31:0] value,     // 32-bit value to display (hex)
    input  wire [7:0]  enable,    // which digits to enable (active high)
    output reg  [6:0]  seg,       // CA..CG (active low)
    output reg         dp,        // decimal point (active low)
    output reg  [7:0]  an         // anodes (active low)
);

    reg [17:0] refresh_counter;
    wire [2:0] digit_sel;
    reg  [3:0] hex_digit;

    assign digit_sel = refresh_counter[17:15];

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n)
            refresh_counter <= 0;
        else
            refresh_counter <= refresh_counter + 1;
    end

    // Select active digit and extract hex nibble
    always @(*) begin
        an  = 8'hFF;  // all off by default
        dp  = 1'b1;   // dp off
        hex_digit = 4'd0;

        case (digit_sel)
            3'd0: begin hex_digit = value[3:0];   if (enable[0]) an[0] = 0; end
            3'd1: begin hex_digit = value[7:4];   if (enable[1]) an[1] = 0; end
            3'd2: begin hex_digit = value[11:8];  if (enable[2]) an[2] = 0; end
            3'd3: begin hex_digit = value[15:12]; if (enable[3]) an[3] = 0; end
            3'd4: begin hex_digit = value[19:16]; if (enable[4]) an[4] = 0; end
            3'd5: begin hex_digit = value[23:20]; if (enable[5]) an[5] = 0; end
            3'd6: begin hex_digit = value[27:24]; if (enable[6]) an[6] = 0; end
            3'd7: begin hex_digit = value[31:28]; if (enable[7]) an[7] = 0; end
        endcase
    end

    // Hex to 7-segment decoder (active low: 0 = on)
    always @(*) begin
        case (hex_digit)
            4'h0: seg = 7'b0000001;
            4'h1: seg = 7'b1001111;
            4'h2: seg = 7'b0010010;
            4'h3: seg = 7'b0000110;
            4'h4: seg = 7'b1001100;
            4'h5: seg = 7'b0100100;
            4'h6: seg = 7'b0100000;
            4'h7: seg = 7'b0001111;
            4'h8: seg = 7'b0000000;
            4'h9: seg = 7'b0000100;
            4'hA: seg = 7'b0001000;
            4'hB: seg = 7'b1100000;
            4'hC: seg = 7'b0110001;
            4'hD: seg = 7'b1000010;
            4'hE: seg = 7'b0110000;
            4'hF: seg = 7'b0111000;
            default: seg = 7'b1111111;
        endcase
    end
endmodule
```

**Step 2: Commit**

```bash
git add dcl-fpga/rtl/seven_seg.v
git commit -m "feat(fpga): implement 7-segment display driver for Nexys 4 DDR"
```

---

### Task 8: Command Dispatcher (`cmd_dispatch.v`)

**Files:**
- Create: `dcl-fpga/rtl/cmd_dispatch.v`

**Step 1: Implement the command dispatcher FSM**

Create `dcl-fpga/rtl/cmd_dispatch.v`:

```verilog
`timescale 1ns / 1ps
// Command Dispatcher — parses UART commands [CMD][LEN][PAYLOAD]
// Routes to compute units and sends responses via UART TX
// Protocol defined in design doc: 7 commands (0x01–0x07)

module cmd_dispatch (
    input  wire        clk,
    input  wire        rst_n,

    // UART RX interface
    input  wire [7:0]  rx_data,
    input  wire        rx_valid,

    // UART TX interface
    output reg  [7:0]  tx_data,
    output reg         tx_send,
    input  wire        tx_busy,

    // GCD unit interface
    output reg  [63:0] gcd_a, gcd_b,
    output reg         gcd_start,
    input  wire [63:0] gcd_result,
    input  wire        gcd_done,
    input  wire        gcd_coprime,

    // Power Map unit interface
    output reg  [63:0] pm_x,
    output reg  [31:0] pm_m,
    output reg  [63:0] pm_modulus,
    output reg         pm_start,
    input  wire [63:0] pm_result,
    input  wire        pm_done,

    // Coprime checker interface
    output reg  [7:0]  cc_num_edges,
    output reg         cc_start,
    input  wire        cc_all_coprime,
    input  wire        cc_done,
    input  wire [7:0]  cc_fail_edge,

    // BRAM write ports (for STORE_LABEL, STORE_EDGE)
    output reg  [4:0]  bram_label_waddr,
    output reg  [63:0] bram_label_wdata,
    output reg         bram_label_we,

    output reg  [7:0]  bram_edge_waddr,
    output reg  [7:0]  bram_edge_u_wdata,
    output reg  [7:0]  bram_edge_v_wdata,
    output reg         bram_edge_we,

    // Status
    output reg  [7:0]  n_labels,
    output reg  [7:0]  n_edges,

    // Busy indicator
    output reg         busy
);

    // Commands
    localparam CMD_GCD           = 8'h01,
               CMD_POWER_MAP    = 8'h02,
               CMD_STORE_LABEL  = 8'h03,
               CMD_STORE_EDGE   = 8'h04,
               CMD_CHECK_COPRIME = 8'h05,
               CMD_EVOLVE       = 8'h06,
               CMD_STATUS       = 8'h07;

    localparam S_IDLE      = 4'd0,
               S_CMD       = 4'd1,
               S_LEN       = 4'd2,
               S_PAYLOAD   = 4'd3,
               S_EXECUTE   = 4'd4,
               S_WAIT      = 4'd5,
               S_RESPOND   = 4'd6,
               S_TX_WAIT   = 4'd7;

    reg [3:0]  state;
    reg [7:0]  cmd;
    reg [7:0]  payload_len;
    reg [7:0]  payload [0:31];  // max 32 bytes payload buffer
    reg [7:0]  payload_idx;

    // Response buffer
    reg [7:0]  resp [0:31];
    reg [7:0]  resp_len;
    reg [7:0]  resp_idx;

    integer i;

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state        <= S_IDLE;
            busy         <= 0;
            tx_send      <= 0;
            tx_data      <= 0;
            gcd_start    <= 0;
            pm_start     <= 0;
            cc_start     <= 0;
            bram_label_we <= 0;
            bram_edge_we  <= 0;
            n_labels     <= 0;
            n_edges      <= 0;
            cmd          <= 0;
            payload_len  <= 0;
            payload_idx  <= 0;
            resp_len     <= 0;
            resp_idx     <= 0;
            for (i = 0; i < 32; i = i + 1) begin
                payload[i] <= 0;
                resp[i]    <= 0;
            end
        end else begin
            tx_send       <= 0;
            gcd_start     <= 0;
            pm_start      <= 0;
            cc_start      <= 0;
            bram_label_we <= 0;
            bram_edge_we  <= 0;

            case (state)
                S_IDLE: begin
                    busy <= 0;
                    if (rx_valid) begin
                        cmd   <= rx_data;
                        state <= S_LEN;
                    end
                end

                S_LEN: begin
                    if (rx_valid) begin
                        payload_len <= rx_data;
                        payload_idx <= 0;
                        if (rx_data == 0)
                            state <= S_EXECUTE;
                        else
                            state <= S_PAYLOAD;
                    end
                end

                S_PAYLOAD: begin
                    if (rx_valid) begin
                        if (payload_idx < 32)
                            payload[payload_idx] <= rx_data;
                        if (payload_idx + 1 >= payload_len)
                            state <= S_EXECUTE;
                        payload_idx <= payload_idx + 1;
                    end
                end

                S_EXECUTE: begin
                    busy <= 1;
                    case (cmd)
                        CMD_GCD: begin
                            // Payload: a[8B] b[8B] (little-endian)
                            gcd_a <= {payload[7], payload[6], payload[5], payload[4],
                                      payload[3], payload[2], payload[1], payload[0]};
                            gcd_b <= {payload[15], payload[14], payload[13], payload[12],
                                      payload[11], payload[10], payload[9], payload[8]};
                            gcd_start <= 1;
                            state <= S_WAIT;
                        end

                        CMD_POWER_MAP: begin
                            // Payload: x[8B] m[4B] mod[8B]
                            pm_x <= {payload[7], payload[6], payload[5], payload[4],
                                     payload[3], payload[2], payload[1], payload[0]};
                            pm_m <= {payload[11], payload[10], payload[9], payload[8]};
                            pm_modulus <= {payload[19], payload[18], payload[17], payload[16],
                                           payload[15], payload[14], payload[13], payload[12]};
                            pm_start <= 1;
                            state <= S_WAIT;
                        end

                        CMD_STORE_LABEL: begin
                            // Payload: idx[1B] label[8B]
                            bram_label_waddr <= payload[0][4:0];
                            bram_label_wdata <= {payload[8], payload[7], payload[6], payload[5],
                                                  payload[4], payload[3], payload[2], payload[1]};
                            bram_label_we <= 1;
                            if (payload[0] >= n_labels)
                                n_labels <= payload[0] + 1;
                            // ACK
                            resp[0] <= 8'h01;
                            resp_len <= 1;
                            resp_idx <= 0;
                            state <= S_RESPOND;
                        end

                        CMD_STORE_EDGE: begin
                            // Payload: idx[1B] u[1B] v[1B]
                            bram_edge_waddr   <= payload[0];
                            bram_edge_u_wdata <= payload[1];
                            bram_edge_v_wdata <= payload[2];
                            bram_edge_we <= 1;
                            if (payload[0] >= n_edges)
                                n_edges <= payload[0] + 1;
                            resp[0] <= 8'h01;
                            resp_len <= 1;
                            resp_idx <= 0;
                            state <= S_RESPOND;
                        end

                        CMD_CHECK_COPRIME: begin
                            // Payload: num_edges[1B]
                            cc_num_edges <= payload[0];
                            cc_start <= 1;
                            state <= S_WAIT;
                        end

                        CMD_STATUS: begin
                            // No payload
                            resp[0] <= n_labels;
                            resp[1] <= n_edges;
                            resp_len <= 2;
                            resp_idx <= 0;
                            state <= S_RESPOND;
                        end

                        default: begin
                            // Unknown command — ignore
                            state <= S_IDLE;
                        end
                    endcase
                end

                S_WAIT: begin
                    // Wait for compute unit to finish
                    case (cmd)
                        CMD_GCD: begin
                            if (gcd_done) begin
                                // Response: gcd[8B] coprime[1B]
                                {resp[7], resp[6], resp[5], resp[4],
                                 resp[3], resp[2], resp[1], resp[0]} <= gcd_result;
                                resp[8] <= {7'd0, gcd_coprime};
                                resp_len <= 9;
                                resp_idx <= 0;
                                state <= S_RESPOND;
                            end
                        end

                        CMD_POWER_MAP: begin
                            if (pm_done) begin
                                {resp[7], resp[6], resp[5], resp[4],
                                 resp[3], resp[2], resp[1], resp[0]} <= pm_result;
                                resp_len <= 8;
                                resp_idx <= 0;
                                state <= S_RESPOND;
                            end
                        end

                        CMD_CHECK_COPRIME: begin
                            if (cc_done) begin
                                resp[0] <= {7'd0, cc_all_coprime};
                                resp[1] <= cc_fail_edge;
                                resp_len <= 2;
                                resp_idx <= 0;
                                state <= S_RESPOND;
                            end
                        end

                        default: state <= S_IDLE;
                    endcase
                end

                S_RESPOND: begin
                    if (!tx_busy) begin
                        tx_data <= resp[resp_idx];
                        tx_send <= 1;
                        state   <= S_TX_WAIT;
                    end
                end

                S_TX_WAIT: begin
                    // Wait for TX to accept the byte
                    if (!tx_busy && resp_idx + 1 >= resp_len) begin
                        state <= S_IDLE;
                    end else if (!tx_busy) begin
                        resp_idx <= resp_idx + 1;
                        state    <= S_RESPOND;
                    end
                end
            endcase
        end
    end
endmodule
```

**Step 2: Commit**

```bash
git add dcl-fpga/rtl/cmd_dispatch.v
git commit -m "feat(fpga): implement UART command dispatcher FSM"
```

---

### Task 9: Demo Controller (`demo_ctrl.v`)

**Files:**
- Create: `dcl-fpga/rtl/demo_ctrl.v`

**Step 1: Implement the standalone demo controller**

Create `dcl-fpga/rtl/demo_ctrl.v`:

```verilog
`timescale 1ns / 1ps
// Standalone demo controller for Nexys 4 DDR
// SW15=1 activates demo mode
// BTNC: GCD demo — SW[7:0]=a, SW[14:8]=b → 7-seg shows GCD, LD0=coprime
// BTNU: Power Map demo — SW[7:0]=base, SW[11:8]=exp → 7-seg shows result
// BTND: Coprime Check — hardcoded P_5, evolves per press
// LD15 RGB: Green=idle, Blue=processing, Red=error

module demo_ctrl (
    input  wire        clk,
    input  wire        rst_n,
    input  wire        demo_mode,     // SW15
    input  wire [14:0] sw,            // SW[14:0]
    input  wire        btnc,          // GCD demo
    input  wire        btnu,          // Power map demo
    input  wire        btnd,          // Coprime check demo

    // GCD unit interface
    output reg  [63:0] gcd_a, gcd_b,
    output reg         gcd_start,
    input  wire [63:0] gcd_result,
    input  wire        gcd_done,
    input  wire        gcd_coprime,

    // Power map unit interface
    output reg  [63:0] pm_x,
    output reg  [31:0] pm_m,
    output reg  [63:0] pm_modulus,
    output reg         pm_start,
    input  wire [63:0] pm_result,
    input  wire        pm_done,

    // Display outputs
    output reg  [31:0] display_value,
    output reg  [7:0]  display_enable,
    output reg  [15:0] led,
    output reg  [2:0]  led16_rgb,    // {R, G, B}

    // State
    output reg         active
);

    localparam S_IDLE       = 3'd0,
               S_GCD_WAIT   = 3'd1,
               S_PM_WAIT    = 3'd2,
               S_CP_WAIT    = 3'd3;

    reg [2:0]  state;
    reg [19:0] debounce_c, debounce_u, debounce_d;
    reg        btnc_prev, btnu_prev, btnd_prev;
    wire       btnc_edge, btnu_edge, btnd_edge;

    // Edge detection with debounce
    assign btnc_edge = (debounce_c == 20'hFFFFF) && !btnc_prev;
    assign btnu_edge = (debounce_u == 20'hFFFFF) && !btnu_prev;
    assign btnd_edge = (debounce_d == 20'hFFFFF) && !btnd_prev;

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            debounce_c <= 0; debounce_u <= 0; debounce_d <= 0;
            btnc_prev <= 0; btnu_prev <= 0; btnd_prev <= 0;
        end else begin
            // Debounce counters
            debounce_c <= btnc ? (debounce_c + (debounce_c != 20'hFFFFF)) : 0;
            debounce_u <= btnu ? (debounce_u + (debounce_u != 20'hFFFFF)) : 0;
            debounce_d <= btnd ? (debounce_d + (debounce_d != 20'hFFFFF)) : 0;
            btnc_prev <= (debounce_c == 20'hFFFFF);
            btnu_prev <= (debounce_u == 20'hFFFFF);
            btnd_prev <= (debounce_d == 20'hFFFFF);
        end
    end

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state          <= S_IDLE;
            active         <= 0;
            gcd_start      <= 0;
            pm_start       <= 0;
            display_value  <= 0;
            display_enable <= 8'hFF;
            led            <= 0;
            led16_rgb      <= 3'b010;  // green = idle
        end else begin
            gcd_start <= 0;
            pm_start  <= 0;

            if (!demo_mode) begin
                active <= 0;
                state  <= S_IDLE;
            end else begin
                active <= 1;

                case (state)
                    S_IDLE: begin
                        led16_rgb <= 3'b010;  // green

                        if (btnc_edge) begin
                            // GCD demo: a = SW[7:0], b = SW[14:8]
                            gcd_a     <= {56'd0, sw[7:0]};
                            gcd_b     <= {57'd0, sw[14:8]};
                            gcd_start <= 1;
                            led16_rgb <= 3'b001;  // blue
                            state     <= S_GCD_WAIT;
                        end else if (btnu_edge) begin
                            // Power Map: base = SW[7:0], exp = SW[11:8]
                            pm_x       <= {56'd0, sw[7:0]};
                            pm_m       <= {28'd0, sw[11:8]};
                            pm_modulus <= 64'd0;  // unbounded
                            pm_start   <= 1;
                            led16_rgb  <= 3'b001;
                            state      <= S_PM_WAIT;
                        end
                    end

                    S_GCD_WAIT: begin
                        if (gcd_done) begin
                            display_value  <= gcd_result[31:0];
                            display_enable <= 8'hFF;
                            led[0]         <= gcd_coprime;
                            led16_rgb      <= 3'b010;
                            state          <= S_IDLE;
                        end
                    end

                    S_PM_WAIT: begin
                        if (pm_done) begin
                            display_value  <= pm_result[31:0];
                            display_enable <= 8'hFF;
                            led16_rgb      <= 3'b010;
                            state          <= S_IDLE;
                        end
                    end

                    default: state <= S_IDLE;
                endcase
            end
        end
    end
endmodule
```

**Step 2: Commit**

```bash
git add dcl-fpga/rtl/demo_ctrl.v
git commit -m "feat(fpga): implement standalone demo controller"
```

---

### Task 10: Top Module (`dcl_top.v`)

**Files:**
- Create: `dcl-fpga/rtl/dcl_top.v`

**Step 1: Implement the top module**

Create `dcl-fpga/rtl/dcl_top.v`:

```verilog
`timescale 1ns / 1ps
// DCL Top Module — Nexys 4 DDR (XC7A100T-1CSG324C)
// Integrates: GCD, MulMod, PowerMap, CoprimeChecker, UART, Demo, 7-Seg

module dcl_top (
    input  wire        CLK100MHZ,
    input  wire        CPU_RESETN,    // active-low reset

    // UART
    input  wire        UART_TXD_IN,   // PC → FPGA (RX)
    output wire        UART_RXD_OUT,  // FPGA → PC (TX)

    // Switches and buttons
    input  wire [15:0] SW,
    input  wire        BTNC, BTNU, BTND, BTNL, BTNR,

    // LEDs
    output wire [15:0] LED,
    output wire [2:0]  LED16_R, LED16_G, LED16_B,

    // 7-segment display
    output wire [6:0]  SEG,           // CA..CG
    output wire        DP,
    output wire [7:0]  AN
);

    wire clk   = CLK100MHZ;
    wire rst_n = CPU_RESETN;

    // ---- UART ----
    wire [7:0] rx_data;
    wire       rx_valid;
    wire [7:0] tx_data;
    wire       tx_send;
    wire       tx_busy;

    uart_rx u_uart_rx (
        .clk(clk), .rst_n(rst_n), .rx(UART_TXD_IN),
        .data(rx_data), .valid(rx_valid)
    );

    uart_tx u_uart_tx (
        .clk(clk), .rst_n(rst_n), .data(tx_data), .send(tx_send),
        .tx(UART_RXD_OUT), .busy(tx_busy)
    );

    // ---- GCD Unit ----
    wire [63:0] gcd_a_cmd, gcd_b_cmd, gcd_a_demo, gcd_b_demo;
    wire        gcd_start_cmd, gcd_start_demo;
    wire [63:0] gcd_result;
    wire        gcd_done, gcd_coprime;

    wire demo_active;
    wire [63:0] gcd_a_mux = demo_active ? gcd_a_demo : gcd_a_cmd;
    wire [63:0] gcd_b_mux = demo_active ? gcd_b_demo : gcd_b_cmd;
    wire        gcd_start_mux = demo_active ? gcd_start_demo : gcd_start_cmd;

    gcd_unit u_gcd (
        .clk(clk), .rst_n(rst_n), .start(gcd_start_mux),
        .a(gcd_a_mux), .b(gcd_b_mux),
        .result(gcd_result), .done(gcd_done), .is_coprime(gcd_coprime)
    );

    // ---- Power Map Unit (includes mulmod_64) ----
    wire [63:0] pm_x_cmd, pm_x_demo;
    wire [31:0] pm_m_cmd, pm_m_demo;
    wire [63:0] pm_mod_cmd, pm_mod_demo;
    wire        pm_start_cmd, pm_start_demo;
    wire [63:0] pm_result;
    wire        pm_done;

    wire [63:0] pm_x_mux   = demo_active ? pm_x_demo   : pm_x_cmd;
    wire [31:0] pm_m_mux   = demo_active ? pm_m_demo   : pm_m_cmd;
    wire [63:0] pm_mod_mux = demo_active ? pm_mod_demo : pm_mod_cmd;
    wire        pm_start_mux = demo_active ? pm_start_demo : pm_start_cmd;

    power_map_unit u_power_map (
        .clk(clk), .rst_n(rst_n), .start(pm_start_mux),
        .x(pm_x_mux), .m(pm_m_mux), .modulus(pm_mod_mux),
        .result(pm_result), .done(pm_done)
    );

    // ---- BRAM (Labels and Edges) ----
    // Labels BRAM: 32 × 64-bit, dual-port (write from cmd_dispatch, read from coprime_checker)
    reg  [63:0] label_mem [0:31];
    wire [4:0]  label_raddr;   // from coprime_checker
    wire [4:0]  label_waddr;   // from cmd_dispatch
    wire [63:0] label_wdata;
    wire        label_we;
    reg  [63:0] label_rdata;

    always @(posedge clk) begin
        if (label_we) label_mem[label_waddr] <= label_wdata;
        label_rdata <= label_mem[label_raddr];
    end

    // Edges BRAM: 256 × (u8, u8)
    reg  [7:0] edge_u_mem [0:255];
    reg  [7:0] edge_v_mem [0:255];
    wire [7:0] edge_raddr;
    wire [7:0] edge_waddr;
    wire [7:0] edge_u_wdata, edge_v_wdata;
    wire       edge_we;
    reg  [7:0] edge_u_rdata, edge_v_rdata;

    always @(posedge clk) begin
        if (edge_we) begin
            edge_u_mem[edge_waddr] <= edge_u_wdata;
            edge_v_mem[edge_waddr] <= edge_v_wdata;
        end
        edge_u_rdata <= edge_u_mem[edge_raddr];
        edge_v_rdata <= edge_v_mem[edge_raddr];
    end

    // ---- Coprime Checker ----
    wire [7:0]  cc_num_edges;
    wire        cc_start;
    wire        cc_all_coprime, cc_done;
    wire [7:0]  cc_fail_edge;

    coprime_checker u_coprime (
        .clk(clk), .rst_n(rst_n), .start(cc_start),
        .num_edges(cc_num_edges),
        .label_data(label_rdata), .label_addr(label_raddr),
        .edge_u_data(edge_u_rdata), .edge_v_data(edge_v_rdata),
        .edge_addr(edge_raddr),
        .all_coprime(cc_all_coprime), .done(cc_done), .fail_edge(cc_fail_edge)
    );

    // ---- Command Dispatcher ----
    wire cmd_busy;
    wire [7:0] cmd_n_labels, cmd_n_edges;

    cmd_dispatch u_cmd (
        .clk(clk), .rst_n(rst_n),
        .rx_data(rx_data), .rx_valid(rx_valid && !SW[15]),
        .tx_data(tx_data), .tx_send(tx_send), .tx_busy(tx_busy),
        .gcd_a(gcd_a_cmd), .gcd_b(gcd_b_cmd), .gcd_start(gcd_start_cmd),
        .gcd_result(gcd_result), .gcd_done(gcd_done), .gcd_coprime(gcd_coprime),
        .pm_x(pm_x_cmd), .pm_m(pm_m_cmd), .pm_modulus(pm_mod_cmd),
        .pm_start(pm_start_cmd), .pm_result(pm_result), .pm_done(pm_done),
        .cc_num_edges(cc_num_edges), .cc_start(cc_start),
        .cc_all_coprime(cc_all_coprime), .cc_done(cc_done), .cc_fail_edge(cc_fail_edge),
        .bram_label_waddr(label_waddr), .bram_label_wdata(label_wdata), .bram_label_we(label_we),
        .bram_edge_waddr(edge_waddr), .bram_edge_u_wdata(edge_u_wdata),
        .bram_edge_v_wdata(edge_v_wdata), .bram_edge_we(edge_we),
        .n_labels(cmd_n_labels), .n_edges(cmd_n_edges),
        .busy(cmd_busy)
    );

    // ---- Demo Controller ----
    wire [31:0] demo_display;
    wire [7:0]  demo_enable;
    wire [15:0] demo_led;
    wire [2:0]  demo_rgb;

    demo_ctrl u_demo (
        .clk(clk), .rst_n(rst_n), .demo_mode(SW[15]), .sw(SW[14:0]),
        .btnc(BTNC), .btnu(BTNU), .btnd(BTND),
        .gcd_a(gcd_a_demo), .gcd_b(gcd_b_demo), .gcd_start(gcd_start_demo),
        .gcd_result(gcd_result), .gcd_done(gcd_done), .gcd_coprime(gcd_coprime),
        .pm_x(pm_x_demo), .pm_m(pm_m_demo), .pm_modulus(pm_mod_demo),
        .pm_start(pm_start_demo), .pm_result(pm_result), .pm_done(pm_done),
        .display_value(demo_display), .display_enable(demo_enable),
        .led(demo_led), .led16_rgb(demo_rgb),
        .active(demo_active)
    );

    // ---- 7-Segment Display ----
    wire [31:0] seg_value  = demo_active ? demo_display : 32'd0;
    wire [7:0]  seg_enable = demo_active ? demo_enable : 8'h00;

    seven_seg u_seven_seg (
        .clk(clk), .rst_n(rst_n),
        .value(seg_value), .enable(seg_enable),
        .seg(SEG), .dp(DP), .an(AN)
    );

    // ---- LED outputs ----
    assign LED = demo_active ? demo_led : {cmd_busy, 7'd0, cmd_n_labels};
    assign LED16_R = demo_active ? {3{demo_rgb[2]}} : {3{cmd_busy}};
    assign LED16_G = demo_active ? {3{demo_rgb[1]}} : {3{!cmd_busy}};
    assign LED16_B = demo_active ? {3{demo_rgb[0]}} : 3'd0;

endmodule
```

**Step 2: Commit**

```bash
git add dcl-fpga/rtl/dcl_top.v
git commit -m "feat(fpga): implement top module integrating all DCL compute units"
```

---

### Task 11: Full System Testbench (`tb_dcl_top.v`)

**Files:**
- Create: `dcl-fpga/tb/tb_dcl_top.v`

**Step 1: Write the system testbench**

Create `dcl-fpga/tb/tb_dcl_top.v`:

```verilog
`timescale 1ns / 1ps

module tb_dcl_top;
    reg         clk, rst_n;
    reg         uart_rx_pin;
    wire        uart_tx_pin;
    reg  [15:0] sw;
    reg         btnc, btnu, btnd, btnl, btnr;
    wire [15:0] led;
    wire [6:0]  seg;
    wire        dp;
    wire [7:0]  an;
    wire [2:0]  led16_r, led16_g, led16_b;

    dcl_top uut (
        .CLK100MHZ(clk), .CPU_RESETN(rst_n),
        .UART_TXD_IN(uart_rx_pin), .UART_RXD_OUT(uart_tx_pin),
        .SW(sw),
        .BTNC(btnc), .BTNU(btnu), .BTND(btnd), .BTNL(btnl), .BTNR(btnr),
        .LED(led),
        .LED16_R(led16_r), .LED16_G(led16_g), .LED16_B(led16_b),
        .SEG(seg), .DP(dp), .AN(an)
    );

    always #5 clk = ~clk;  // 100 MHz

    // UART bit period at 115200 baud = 8680 ns
    localparam BAUD_PERIOD = 8680;

    // Task: send one byte via UART
    task uart_send_byte(input [7:0] data);
        integer i;
        begin
            // Start bit
            uart_rx_pin = 0;
            #(BAUD_PERIOD);
            // 8 data bits (LSB first)
            for (i = 0; i < 8; i = i + 1) begin
                uart_rx_pin = data[i];
                #(BAUD_PERIOD);
            end
            // Stop bit
            uart_rx_pin = 1;
            #(BAUD_PERIOD);
        end
    endtask

    initial begin
        clk = 0; rst_n = 0;
        uart_rx_pin = 1;  // idle
        sw = 16'd0;
        btnc = 0; btnu = 0; btnd = 0; btnl = 0; btnr = 0;

        #100 rst_n = 1;

        // ---- Test 1: UART GCD command ----
        // CMD=0x01, LEN=16, a=12 (LE), b=8 (LE)
        $display("Sending GCD command: gcd(12, 8)");
        uart_send_byte(8'h01);  // CMD
        uart_send_byte(8'd16);  // LEN
        // a = 12 (little-endian 8 bytes)
        uart_send_byte(8'd12); uart_send_byte(0); uart_send_byte(0); uart_send_byte(0);
        uart_send_byte(0); uart_send_byte(0); uart_send_byte(0); uart_send_byte(0);
        // b = 8
        uart_send_byte(8'd8); uart_send_byte(0); uart_send_byte(0); uart_send_byte(0);
        uart_send_byte(0); uart_send_byte(0); uart_send_byte(0); uart_send_byte(0);

        // Wait for processing + response
        #500000;

        // ---- Test 2: Demo mode GCD ----
        $display("Demo mode: GCD of SW[7:0]=12, SW[14:8]=8");
        sw = 16'h8000 | (8'd8 << 8) | 8'd12;  // SW15=1, SW[14:8]=8, SW[7:0]=12
        #100;
        btnc = 1;
        #(21 * 100_000);  // debounce ~2ms
        btnc = 0;
        #200000;

        $display("=== System Testbench Complete ===");
        $finish;
    end
endmodule
```

**Step 2: Run system testbench**

```bash
cd dcl-fpga
iverilog -g2012 -o tb/tb_dcl_top.vvp \
  rtl/gcd_unit.v rtl/mulmod_64.v rtl/power_map_unit.v rtl/coprime_checker.v \
  rtl/uart_rx.v rtl/uart_tx.v rtl/cmd_dispatch.v rtl/demo_ctrl.v rtl/seven_seg.v \
  rtl/dcl_top.v tb/tb_dcl_top.v \
  && vvp tb/tb_dcl_top.vvp
```
Expected: Testbench completes without errors.

**Step 3: Commit**

```bash
git add dcl-fpga/tb/tb_dcl_top.v
git commit -m "feat(fpga): add full system testbench"
```

---

### Task 12: Nexys 4 DDR Constraints (`nexys4ddr.xdc`)

**Files:**
- Create: `dcl-fpga/constraints/nexys4ddr.xdc`

**Step 1: Create the constraints file**

Create `dcl-fpga/constraints/nexys4ddr.xdc` with all pin assignments from the Nexys 4 DDR reference:

```tcl
## Clock signal (100 MHz)
set_property -dict { PACKAGE_PIN E3    IOSTANDARD LVCMOS33 } [get_ports { CLK100MHZ }];
create_clock -add -name sys_clk_pin -period 10.00 -waveform {0 5} [get_ports { CLK100MHZ }];

## Reset (active-low)
set_property -dict { PACKAGE_PIN C12   IOSTANDARD LVCMOS33 } [get_ports { CPU_RESETN }];

## Switches
set_property -dict { PACKAGE_PIN J15   IOSTANDARD LVCMOS33 } [get_ports { SW[0] }];
set_property -dict { PACKAGE_PIN L16   IOSTANDARD LVCMOS33 } [get_ports { SW[1] }];
set_property -dict { PACKAGE_PIN M13   IOSTANDARD LVCMOS33 } [get_ports { SW[2] }];
set_property -dict { PACKAGE_PIN R15   IOSTANDARD LVCMOS33 } [get_ports { SW[3] }];
set_property -dict { PACKAGE_PIN R17   IOSTANDARD LVCMOS33 } [get_ports { SW[4] }];
set_property -dict { PACKAGE_PIN T18   IOSTANDARD LVCMOS33 } [get_ports { SW[5] }];
set_property -dict { PACKAGE_PIN U18   IOSTANDARD LVCMOS33 } [get_ports { SW[6] }];
set_property -dict { PACKAGE_PIN R13   IOSTANDARD LVCMOS33 } [get_ports { SW[7] }];
set_property -dict { PACKAGE_PIN T8    IOSTANDARD LVCMOS18 } [get_ports { SW[8] }];
set_property -dict { PACKAGE_PIN U8    IOSTANDARD LVCMOS18 } [get_ports { SW[9] }];
set_property -dict { PACKAGE_PIN R16   IOSTANDARD LVCMOS33 } [get_ports { SW[10] }];
set_property -dict { PACKAGE_PIN T13   IOSTANDARD LVCMOS33 } [get_ports { SW[11] }];
set_property -dict { PACKAGE_PIN H6    IOSTANDARD LVCMOS33 } [get_ports { SW[12] }];
set_property -dict { PACKAGE_PIN U12   IOSTANDARD LVCMOS33 } [get_ports { SW[13] }];
set_property -dict { PACKAGE_PIN U11   IOSTANDARD LVCMOS33 } [get_ports { SW[14] }];
set_property -dict { PACKAGE_PIN V10   IOSTANDARD LVCMOS33 } [get_ports { SW[15] }];

## LEDs
set_property -dict { PACKAGE_PIN H17   IOSTANDARD LVCMOS33 } [get_ports { LED[0] }];
set_property -dict { PACKAGE_PIN K15   IOSTANDARD LVCMOS33 } [get_ports { LED[1] }];
set_property -dict { PACKAGE_PIN J13   IOSTANDARD LVCMOS33 } [get_ports { LED[2] }];
set_property -dict { PACKAGE_PIN N14   IOSTANDARD LVCMOS33 } [get_ports { LED[3] }];
set_property -dict { PACKAGE_PIN R18   IOSTANDARD LVCMOS33 } [get_ports { LED[4] }];
set_property -dict { PACKAGE_PIN V17   IOSTANDARD LVCMOS33 } [get_ports { LED[5] }];
set_property -dict { PACKAGE_PIN U17   IOSTANDARD LVCMOS33 } [get_ports { LED[6] }];
set_property -dict { PACKAGE_PIN U16   IOSTANDARD LVCMOS33 } [get_ports { LED[7] }];
set_property -dict { PACKAGE_PIN V16   IOSTANDARD LVCMOS33 } [get_ports { LED[8] }];
set_property -dict { PACKAGE_PIN T15   IOSTANDARD LVCMOS33 } [get_ports { LED[9] }];
set_property -dict { PACKAGE_PIN U14   IOSTANDARD LVCMOS33 } [get_ports { LED[10] }];
set_property -dict { PACKAGE_PIN T16   IOSTANDARD LVCMOS33 } [get_ports { LED[11] }];
set_property -dict { PACKAGE_PIN V15   IOSTANDARD LVCMOS33 } [get_ports { LED[12] }];
set_property -dict { PACKAGE_PIN V14   IOSTANDARD LVCMOS33 } [get_ports { LED[13] }];
set_property -dict { PACKAGE_PIN V12   IOSTANDARD LVCMOS33 } [get_ports { LED[14] }];
set_property -dict { PACKAGE_PIN V11   IOSTANDARD LVCMOS33 } [get_ports { LED[15] }];

## RGB LED 16
set_property -dict { PACKAGE_PIN N15   IOSTANDARD LVCMOS33 } [get_ports { LED16_R[0] }];
set_property -dict { PACKAGE_PIN M16   IOSTANDARD LVCMOS33 } [get_ports { LED16_R[1] }];
set_property -dict { PACKAGE_PIN R12   IOSTANDARD LVCMOS33 } [get_ports { LED16_R[2] }];
set_property -dict { PACKAGE_PIN N16   IOSTANDARD LVCMOS33 } [get_ports { LED16_G[0] }];
set_property -dict { PACKAGE_PIN R11   IOSTANDARD LVCMOS33 } [get_ports { LED16_G[1] }];
set_property -dict { PACKAGE_PIN G14   IOSTANDARD LVCMOS33 } [get_ports { LED16_G[2] }];
set_property -dict { PACKAGE_PIN R14   IOSTANDARD LVCMOS33 } [get_ports { LED16_B[0] }];
set_property -dict { PACKAGE_PIN P14   IOSTANDARD LVCMOS33 } [get_ports { LED16_B[1] }];
set_property -dict { PACKAGE_PIN N14   IOSTANDARD LVCMOS33 } [get_ports { LED16_B[2] }];

## 7-Segment Display
set_property -dict { PACKAGE_PIN T10   IOSTANDARD LVCMOS33 } [get_ports { SEG[0] }];
set_property -dict { PACKAGE_PIN R10   IOSTANDARD LVCMOS33 } [get_ports { SEG[1] }];
set_property -dict { PACKAGE_PIN K16   IOSTANDARD LVCMOS33 } [get_ports { SEG[2] }];
set_property -dict { PACKAGE_PIN K13   IOSTANDARD LVCMOS33 } [get_ports { SEG[3] }];
set_property -dict { PACKAGE_PIN P15   IOSTANDARD LVCMOS33 } [get_ports { SEG[4] }];
set_property -dict { PACKAGE_PIN T11   IOSTANDARD LVCMOS33 } [get_ports { SEG[5] }];
set_property -dict { PACKAGE_PIN L18   IOSTANDARD LVCMOS33 } [get_ports { SEG[6] }];
set_property -dict { PACKAGE_PIN H15   IOSTANDARD LVCMOS33 } [get_ports { DP }];

set_property -dict { PACKAGE_PIN J17   IOSTANDARD LVCMOS33 } [get_ports { AN[0] }];
set_property -dict { PACKAGE_PIN J18   IOSTANDARD LVCMOS33 } [get_ports { AN[1] }];
set_property -dict { PACKAGE_PIN T9    IOSTANDARD LVCMOS33 } [get_ports { AN[2] }];
set_property -dict { PACKAGE_PIN J14   IOSTANDARD LVCMOS33 } [get_ports { AN[3] }];
set_property -dict { PACKAGE_PIN P14   IOSTANDARD LVCMOS33 } [get_ports { AN[4] }];
set_property -dict { PACKAGE_PIN T14   IOSTANDARD LVCMOS33 } [get_ports { AN[5] }];
set_property -dict { PACKAGE_PIN K2    IOSTANDARD LVCMOS33 } [get_ports { AN[6] }];
set_property -dict { PACKAGE_PIN U13   IOSTANDARD LVCMOS33 } [get_ports { AN[7] }];

## Buttons
set_property -dict { PACKAGE_PIN N17   IOSTANDARD LVCMOS33 } [get_ports { BTNC }];
set_property -dict { PACKAGE_PIN M18   IOSTANDARD LVCMOS33 } [get_ports { BTNU }];
set_property -dict { PACKAGE_PIN P17   IOSTANDARD LVCMOS33 } [get_ports { BTND }];
set_property -dict { PACKAGE_PIN P18   IOSTANDARD LVCMOS33 } [get_ports { BTNL }];
set_property -dict { PACKAGE_PIN M17   IOSTANDARD LVCMOS33 } [get_ports { BTNR }];

## USB-UART (onboard FTDI)
set_property -dict { PACKAGE_PIN C4    IOSTANDARD LVCMOS33 } [get_ports { UART_TXD_IN }];
set_property -dict { PACKAGE_PIN D4    IOSTANDARD LVCMOS33 } [get_ports { UART_RXD_OUT }];

## Configuration
set_property CFGBVS VCCO [current_design];
set_property CONFIG_VOLTAGE 3.3 [current_design];
```

**Step 2: Commit**

```bash
git add dcl-fpga/constraints/nexys4ddr.xdc
git commit -m "feat(fpga): add Nexys 4 DDR pin constraints"
```

---

### Task 13: Vivado Build Script (`build.tcl`)

**Files:**
- Create: `dcl-fpga/scripts/build.tcl`

**Step 1: Create the Vivado batch build script**

Create `dcl-fpga/scripts/build.tcl`:

```tcl
# Vivado non-project batch build for DCL-FPGA
# Usage: vivado -mode batch -source scripts/build.tcl
# Target: Nexys 4 DDR — XC7A100T-1CSG324C

set project_name "dcl_fpga"
set part "xc7a100tcsg324-1"
set top "dcl_top"

# Source files
set rtl_files [glob rtl/*.v]
set xdc_file "constraints/nexys4ddr.xdc"

# Create in-memory project
create_project -in_memory -part $part

# Add sources
foreach f $rtl_files {
    read_verilog $f
}
read_xdc $xdc_file

# Synthesis
synth_design -top $top -part $part
report_utilization -file reports/post_synth_util.rpt
report_timing_summary -file reports/post_synth_timing.rpt

# Implementation
opt_design
place_design
route_design
report_utilization -file reports/post_impl_util.rpt
report_timing_summary -file reports/post_impl_timing.rpt

# Generate bitstream
write_bitstream -force output/${project_name}.bit

puts "=== Build complete: output/${project_name}.bit ==="
```

**Step 2: Commit**

```bash
git add dcl-fpga/scripts/build.tcl
git commit -m "feat(fpga): add Vivado non-project batch build script"
```

---

### Task 14: Rust Host Driver (`dcl_fpga_host`)

**Files:**
- Create: `dcl-fpga/host/dcl_fpga_host/Cargo.toml`
- Create: `dcl-fpga/host/dcl_fpga_host/src/lib.rs`

**Step 1: Create Cargo.toml**

Create `dcl-fpga/host/dcl_fpga_host/Cargo.toml`:

```toml
[package]
name = "dcl_fpga_host"
version = "0.1.0"
edition = "2021"
description = "Host-side UART driver for DCL FPGA accelerator"

[dependencies]
serialport = "4"
thiserror = "2"
```

**Step 2: Implement the host driver**

Create `dcl-fpga/host/dcl_fpga_host/src/lib.rs`:

```rust
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
```

**Step 3: Commit**

```bash
git add dcl-fpga/host/dcl_fpga_host/
git commit -m "feat(fpga): implement Rust host driver for UART communication"
```

---

### Task 15: Test Vector Generation

**Files:**
- Create: `dcl-fpga/tb/generate_test_vectors.rs` (standalone Rust script)

**Step 1: Create test vector generator**

This is a standalone Rust script (run with `cargo script` or `rustc`) that generates `.mem` files for Verilog `$readmemh` from `dcl-core` computations.

Create `dcl-fpga/tb/generate_test_vectors.rs`:

```rust
//! Generate .mem test vector files for FPGA testbenches.
//! Run: rustc generate_test_vectors.rs -o gen_vectors && ./gen_vectors
//! Or add as a binary to dcl_fpga_host.

use std::fs::File;
use std::io::Write;

fn gcd(mut a: u64, mut b: u64) -> u64 {
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

fn main() {
    // GCD test vectors: a, b, expected_gcd
    let gcd_cases: Vec<(u64, u64, u64)> = vec![
        (12, 8, 4), (17, 13, 1), (0, 5, 5), (100, 0, 100),
        (0, 0, 0), (1, 1, 1), (7, 13, 1), (6, 9, 3),
        (1024, 512, 512), (104729, 104743, 1),
    ];

    let mut f = File::create("gcd_vectors.mem").unwrap();
    for (a, b, expected) in &gcd_cases {
        assert_eq!(gcd(*a, *b), *expected);
        writeln!(f, "{:016x} {:016x} {:016x}", a, b, expected).unwrap();
    }
    println!("Generated gcd_vectors.mem ({} cases)", gcd_cases.len());
}
```

**Step 2: Commit**

```bash
git add dcl-fpga/tb/generate_test_vectors.rs
git commit -m "feat(fpga): add test vector generator for FPGA verification"
```

---

## Summary

| Task | Module | Est. LUTs | Files |
|------|--------|-----------|-------|
| 1 | Scaffold | — | directories |
| 2 | gcd_unit | ~500 | rtl + tb |
| 3 | mulmod_64 | ~400 | rtl |
| 4 | power_map_unit | ~800 | rtl + tb |
| 5 | coprime_checker | ~300 | rtl + tb |
| 6 | uart_rx + uart_tx | ~200 | rtl |
| 7 | seven_seg | ~100 | rtl |
| 8 | cmd_dispatch | ~500 | rtl |
| 9 | demo_ctrl | ~200 | rtl |
| 10 | dcl_top | ~100 | rtl |
| 11 | tb_dcl_top | — | tb |
| 12 | nexys4ddr.xdc | — | constraints |
| 13 | build.tcl | — | scripts |
| 14 | Rust host driver | — | host crate |
| 15 | Test vectors | — | tb |

**Total estimated: ~3,100 LUTs (4.9% of XC7A100T)**
