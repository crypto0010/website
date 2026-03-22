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

    localparam S_IDLE       = 4'd0,
               S_CHECK_BIT  = 4'd1,
               S_WAIT_MUL   = 4'd2,  // wait for mul_done to clear before polling (multiply)
               S_MULTIPLY   = 4'd3,
               S_WAIT_SQ    = 4'd4,  // wait for mul_done to clear before polling (square)
               S_SQUARE     = 4'd5,
               S_SHIFT      = 4'd6,
               S_DONE       = 4'd7;

    reg [3:0]   state;
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
                        acc  <= 1;
                        base <= x;
                        state <= S_CHECK_BIT;
                    end
                end

                S_CHECK_BIT: begin
                    if (exp == 0) begin
                        // Map 0 -> 1 (convention)
                        result <= (acc == 0) ? 64'd1 : acc;
                        done   <= 1;
                        state  <= S_IDLE;
                    end else if (exp[0]) begin
                        // Bit set: acc = acc * base (mod)
                        mul_a     <= acc;
                        mul_b     <= base;
                        mul_start <= 1;
                        state     <= S_WAIT_MUL;
                    end else begin
                        // Bit clear: skip to square
                        mul_a     <= base;
                        mul_b     <= base;
                        mul_start <= 1;
                        state     <= S_WAIT_SQ;
                    end
                end

                // Wait one cycle for mulmod to register the start and clear done,
                // preventing the stale mul_done race condition.
                S_WAIT_MUL: begin
                    if (!mul_done)
                        state <= S_MULTIPLY;
                end

                S_MULTIPLY: begin
                    if (mul_done) begin
                        acc <= mul_result;
                        // Now square: base = base * base
                        mul_a     <= base;
                        mul_b     <= base;
                        mul_start <= 1;
                        state     <= S_WAIT_SQ;
                    end
                end

                // Wait one cycle for mulmod to register the start and clear done,
                // preventing the stale mul_done race condition.
                S_WAIT_SQ: begin
                    if (!mul_done)
                        state <= S_SQUARE;
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
