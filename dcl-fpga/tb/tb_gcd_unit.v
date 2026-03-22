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
