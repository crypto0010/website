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
    wire        led16_r, led16_g, led16_b;

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
