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
