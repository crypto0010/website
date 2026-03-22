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
