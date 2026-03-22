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
