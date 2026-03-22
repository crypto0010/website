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
