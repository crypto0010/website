`timescale 1ns / 1ps
// Stein's binary GCD — 64-bit iterative FSM
// Algorithm matches dcl-core/src/gcd.rs:
//   if a==0 return b; if b==0 return a;
//   shift = ctz(a|b); a >>= ctz(a);
//   loop { b >>= ctz(b); if a>b swap; b -= a; if b==0 return a<<shift }
//
// Area-optimized: all CTZ and barrel shift operations replaced with
// iterative 1-bit shifts (eliminates priority encoders and barrel shifters).

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

    localparam S_IDLE         = 4'd0,
               S_DONE_ZERO    = 4'd1,
               S_STRIP_COMMON = 4'd2,  // loop: strip shared trailing zeros
               S_STRIP_A      = 4'd3,  // loop: strip ra trailing zeros
               S_STRIP_B      = 4'd4,  // loop: strip rb trailing zeros
               S_LOOP_CMP     = 4'd5,  // compare and swap
               S_LOOP_SUB     = 4'd6,  // subtract
               S_FINISH_SHIFT = 4'd7,  // loop: left-shift ra by 'shift'
               S_FINISH_OUT   = 4'd8;  // output result

    reg [3:0]  state;
    reg [63:0] ra, rb;
    reg [6:0]  shift;  // common shift factor (max 63)

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state      <= S_IDLE;
            result     <= 64'd0;
            done       <= 1'b0;
            is_coprime <= 1'b0;
            ra         <= 64'd0;
            rb         <= 64'd0;
            shift      <= 7'd0;
        end else begin
            case (state)
                S_IDLE: begin
                    done <= 1'b0;
                    if (start) begin
                        ra    <= a;
                        rb    <= b;
                        shift <= 7'd0;
                        if (a == 64'd0 || b == 64'd0)
                            state <= S_DONE_ZERO;
                        else
                            state <= S_STRIP_COMMON;
                    end
                end

                S_DONE_ZERO: begin
                    result     <= ra | rb;
                    is_coprime <= (ra | rb) == 64'd1;
                    done       <= 1'b1;
                    state      <= S_IDLE;
                end

                // Strip common trailing zeros: while both ra,rb are even,
                // shift both right by 1 and increment shift counter.
                S_STRIP_COMMON: begin
                    if (ra[0] | rb[0])
                        state <= S_STRIP_A;
                    else begin
                        ra    <= {1'b0, ra[63:1]};
                        rb    <= {1'b0, rb[63:1]};
                        shift <= shift + 7'd1;
                    end
                end

                // Strip ra trailing zeros: while ra is even, shift right by 1.
                S_STRIP_A: begin
                    if (ra[0])
                        state <= S_STRIP_B;
                    else
                        ra <= {1'b0, ra[63:1]};
                end

                // Strip rb trailing zeros: while rb is even, shift right by 1.
                S_STRIP_B: begin
                    if (rb[0])
                        state <= S_LOOP_CMP;
                    else
                        rb <= {1'b0, rb[63:1]};
                end

                // Compare and swap so ra <= rb.
                S_LOOP_CMP: begin
                    if (ra > rb) begin
                        ra <= rb;
                        rb <= ra;
                    end
                    state <= S_LOOP_SUB;
                end

                // Subtract: rb = rb - ra. If result is zero, done.
                S_LOOP_SUB: begin
                    rb <= rb - ra;
                    if (rb == ra)
                        state <= S_FINISH_SHIFT;
                    else
                        state <= S_STRIP_B;
                end

                // Left-shift ra by 'shift' positions, one bit per cycle.
                S_FINISH_SHIFT: begin
                    if (shift == 7'd0)
                        state <= S_FINISH_OUT;
                    else begin
                        ra    <= {ra[62:0], 1'b0};
                        shift <= shift - 7'd1;
                    end
                end

                // Output final result.
                S_FINISH_OUT: begin
                    result     <= ra;
                    is_coprime <= (ra == 64'd1);
                    done       <= 1'b1;
                    state      <= S_IDLE;
                end

                default: state <= S_IDLE;
            endcase
        end
    end
endmodule
