`timescale 1ns / 1ps
// 64-bit modular multiplication: result = (a * b) mod m
// Algorithm: Russian peasant binary method (same as GPU kernel mulmod)
//   result = 0
//   a = a mod m       (computed iteratively via restoring division)
//   for each bit of b (LSB to MSB):
//     if bit set: result = (result + a) mod m
//     a = (2*a) mod m
//
// When m == 0: saturating multiply (for unbounded power map)
//   Uses 64-bit arithmetic with an overflow flag instead of 128-bit
//   intermediates. Once overflow is detected, result saturates to MAX.
//
// Cycles: 64 (reduction) + 1 (init) + up to 64 (multiply) in modular mode
//         up to 64 in saturating mode

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

    localparam S_IDLE      = 3'd0,
               S_REDUCE    = 3'd1,   // iterative a mod m (restoring division)
               S_CALC_INIT = 3'd2,   // transfer remainder to ra
               S_CALC      = 3'd3,
               S_DONE      = 3'd4;

    reg [2:0]   state;
    reg [63:0]  ra;       // shift register during reduction, running 'a' during calc
    reg [63:0]  rb;       // remaining bits of 'b'
    reg [63:0]  acc;      // accumulator (shared between modular and saturating modes)
    reg         saturating;
    reg         saturated;    // overflow detected in saturating mode
    reg         ra_overflow;  // ra has exceeded 64 bits (saturating mode)
    reg [64:0]  rem;       // 65-bit remainder for iterative modulo reduction
    reg [5:0]   bit_cnt;   // bit counter for reduction (63 downto 0)

    // 65-bit intermediate for accumulation (shared between modes)
    wire [64:0] sum_acc_ra = {1'b0, acc} + {1'b0, ra};
    // 65-bit intermediate for doubling ra in modular mode
    wire [64:0] sum_ra_ra  = {1'b0, ra}  + {1'b0, ra};

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state  <= S_IDLE;
            result <= 0;
            done   <= 0;
            ra <= 0; rb <= 0; acc <= 0;
            saturating  <= 0;
            saturated   <= 0;
            ra_overflow <= 0;
            rem     <= 0;
            bit_cnt <= 0;
        end else begin
            case (state)
                S_IDLE: begin
                    done <= 0;
                    if (start) begin
                        rb  <= b;
                        acc <= 0;
                        if (m == 0) begin
                            // Saturating mode — no reduction needed
                            saturating  <= 1;
                            saturated   <= 0;
                            ra_overflow <= 0;
                            ra    <= a;
                            state <= S_CALC;
                        end else begin
                            // Modular mode — start iterative reduction
                            saturating <= 0;
                            saturated  <= 0;
                            ra      <= a;        // shift register (MSB out first)
                            rem     <= 65'd0;
                            bit_cnt <= 6'd63;
                            state   <= S_REDUCE;
                        end
                    end
                end

                // Iterative modulo reduction: compute a mod m
                // Restoring division, one bit per cycle, MSB first.
                // Invariant: rem < m after each iteration.
                S_REDUCE: begin
                    if ({rem[63:0], ra[63]} >= {1'b0, m})
                        rem <= {rem[63:0], ra[63]} - {1'b0, m};
                    else
                        rem <= {rem[63:0], ra[63]};
                    ra <= {ra[62:0], 1'b0};
                    if (bit_cnt == 0)
                        state <= S_CALC_INIT;
                    else
                        bit_cnt <= bit_cnt - 1;
                end

                // Transfer computed remainder to ra
                S_CALC_INIT: begin
                    ra    <= rem[63:0];
                    state <= S_CALC;
                end

                S_CALC: begin
                    if (rb == 0) begin
                        result <= saturated ? 64'hFFFFFFFF_FFFFFFFF : acc;
                        done   <= 1;
                        state  <= S_IDLE;
                    end else begin
                        if (saturating) begin
                            // Saturating: 64-bit arithmetic with overflow tracking
                            if (!saturated) begin
                                // Accumulate if current bit is set
                                if (rb[0]) begin
                                    if (ra_overflow || sum_acc_ra[64])
                                        saturated <= 1;
                                    else
                                        acc <= sum_acc_ra[63:0];
                                end
                                // Double ra for next iteration
                                if (!ra_overflow) begin
                                    if (ra[63])
                                        ra_overflow <= 1;
                                    ra <= {ra[62:0], 1'b0};
                                end
                            end
                        end else begin
                            // Modular: all ops mod m, using 65-bit intermediates
                            if (rb[0]) begin
                                acc <= (sum_acc_ra >= {1'b0, m}) ?
                                       sum_acc_ra[63:0] - m : sum_acc_ra[63:0];
                            end
                            ra <= (sum_ra_ra >= {1'b0, m}) ?
                                  sum_ra_ra[63:0] - m : sum_ra_ra[63:0];
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
