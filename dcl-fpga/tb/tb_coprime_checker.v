`timescale 1ns / 1ps

module tb_coprime_checker;
    reg         clk, rst_n, start;
    reg  [7:0]  num_edges;

    // BRAM interfaces (directly driven by TB)
    reg  [63:0] label_data;
    wire [4:0]  label_addr;
    reg         label_valid;

    reg  [7:0]  edge_u_data, edge_v_data;
    wire [7:0]  edge_addr;
    reg         edge_valid;

    wire        all_coprime, done;
    wire [7:0]  fail_edge;

    // Simulated BRAM for labels (32 × 64-bit)
    reg [63:0] labels [0:31];
    // Simulated BRAM for edges (256 × 16-bit packed)
    reg [7:0] edges_u [0:255];
    reg [7:0] edges_v [0:255];

    coprime_checker uut (
        .clk(clk), .rst_n(rst_n), .start(start),
        .num_edges(num_edges),
        .label_data(label_data), .label_addr(label_addr),
        .edge_u_data(edge_u_data), .edge_v_data(edge_v_data),
        .edge_addr(edge_addr),
        .all_coprime(all_coprime), .done(done), .fail_edge(fail_edge)
    );

    always #5 clk = ~clk;

    // BRAM read simulation (1-cycle latency)
    always @(posedge clk) begin
        label_data  <= labels[label_addr];
        edge_u_data <= edges_u[edge_addr];
        edge_v_data <= edges_v[edge_addr];
    end

    integer pass_count = 0;
    integer fail_count = 0;
    integer i;

    initial begin
        clk = 0; rst_n = 0; start = 0; num_edges = 0;

        // Initialize BRAM
        for (i = 0; i < 32; i = i + 1) labels[i] = 0;
        for (i = 0; i < 256; i = i + 1) begin
            edges_u[i] = 0; edges_v[i] = 0;
        end

        #20 rst_n = 1;

        // --- Test 1: P_5 with coprime labeling [1,2,3,5,7] ---
        // Edges: (0,1), (1,2), (2,3), (3,4)
        labels[0] = 1; labels[1] = 2; labels[2] = 3;
        labels[3] = 5; labels[4] = 7;
        edges_u[0] = 0; edges_v[0] = 1;
        edges_u[1] = 1; edges_v[1] = 2;
        edges_u[2] = 2; edges_v[2] = 3;
        edges_u[3] = 3; edges_v[3] = 4;

        @(posedge clk);
        num_edges = 4; start = 1;
        @(posedge clk);
        start = 0;
        wait(done);
        @(posedge clk);
        if (all_coprime !== 1) begin
            $display("FAIL: P_5 coprime labeling should pass"); fail_count = fail_count + 1;
        end else begin
            $display("PASS: P_5 coprime labeling"); pass_count = pass_count + 1;
        end

        // --- Test 2: P_4 with non-coprime labeling [6,9,5,7] ---
        labels[0] = 6; labels[1] = 9; labels[2] = 5; labels[3] = 7;
        edges_u[0] = 0; edges_v[0] = 1;
        edges_u[1] = 1; edges_v[1] = 2;
        edges_u[2] = 2; edges_v[2] = 3;

        @(posedge clk);
        num_edges = 3; start = 1;
        @(posedge clk);
        start = 0;
        wait(done);
        @(posedge clk);
        if (all_coprime !== 0) begin
            $display("FAIL: non-coprime labeling should fail"); fail_count = fail_count + 1;
        end else begin
            $display("PASS: non-coprime labeling detected, fail_edge=%0d", fail_edge);
            pass_count = pass_count + 1;
        end

        // --- Test 3: Empty graph (0 edges) ---
        @(posedge clk);
        num_edges = 0; start = 1;
        @(posedge clk);
        start = 0;
        wait(done);
        @(posedge clk);
        if (all_coprime !== 1) begin
            $display("FAIL: empty graph should be coprime"); fail_count = fail_count + 1;
        end else begin
            $display("PASS: empty graph coprime"); pass_count = pass_count + 1;
        end

        $display("\n=== Coprime Checker: %0d PASSED, %0d FAILED ===", pass_count, fail_count);
        $finish;
    end
endmodule
