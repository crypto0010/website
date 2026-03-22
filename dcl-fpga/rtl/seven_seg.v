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
