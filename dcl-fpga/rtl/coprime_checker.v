`timescale 1ns / 1ps
// Sequential coprimality checker — iterates edges, reuses gcd_unit
// Reads labels[] and edges[] from BRAM ports
// Algorithm: for each edge (u,v), compute gcd(labels[u], labels[v])
//   If any gcd != 1, set all_coprime=0 and report fail_edge

module coprime_checker (
    input  wire        clk,
    input  wire        rst_n,
    input  wire        start,
    input  wire [7:0]  num_edges,

    // BRAM read ports
    input  wire [63:0] label_data,
    output reg  [4:0]  label_addr,

    input  wire [7:0]  edge_u_data,
    input  wire [7:0]  edge_v_data,
    output reg  [7:0]  edge_addr,

    output reg         all_coprime,
    output reg         done,
    output reg  [7:0]  fail_edge
);

    localparam S_IDLE     = 3'd0,
               S_RD_EDGE  = 3'd1,  // read edge (u,v) from BRAM
               S_RD_LBL_U = 3'd2,  // request label[u]
               S_RD_LBL_V = 3'd3,  // request label[v]
               S_GCD      = 3'd4,  // run GCD
               S_CHECK    = 3'd5,  // check GCD result
               S_DONE     = 3'd6;

    reg [2:0]  state;
    reg [7:0]  edge_idx;
    reg [63:0] label_u, label_v;
    reg [7:0]  vert_u, vert_v;

    // GCD unit wires
    reg        gcd_start;
    reg [63:0] gcd_a, gcd_b;
    wire [63:0] gcd_result;
    wire        gcd_done, gcd_coprime;

    gcd_unit u_gcd (
        .clk(clk), .rst_n(rst_n), .start(gcd_start),
        .a(gcd_a), .b(gcd_b),
        .result(gcd_result), .done(gcd_done), .is_coprime(gcd_coprime)
    );

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state       <= S_IDLE;
            done        <= 0;
            all_coprime <= 0;
            fail_edge   <= 0;
            edge_idx    <= 0;
            gcd_start   <= 0;
            label_addr  <= 0;
            edge_addr   <= 0;
            label_u     <= 0;
            label_v     <= 0;
            gcd_a       <= 0;
            gcd_b       <= 0;
        end else begin
            gcd_start <= 0;

            case (state)
                S_IDLE: begin
                    done <= 0;
                    if (start) begin
                        if (num_edges == 0) begin
                            all_coprime <= 1;
                            fail_edge   <= 0;
                            done        <= 1;
                            state       <= S_IDLE;
                        end else begin
                            edge_idx    <= 0;
                            all_coprime <= 1;
                            fail_edge   <= 0;
                            edge_addr   <= 0;
                            state       <= S_RD_EDGE;
                        end
                    end
                end

                S_RD_EDGE: begin
                    // Wait 1 cycle for BRAM read latency
                    state <= S_RD_LBL_U;
                end

                S_RD_LBL_U: begin
                    // Edge data available: latch u,v and request label[u]
                    vert_u     <= edge_u_data;
                    vert_v     <= edge_v_data;
                    label_addr <= edge_u_data[4:0];
                    state      <= S_RD_LBL_V;
                end

                S_RD_LBL_V: begin
                    // label[u] available, request label[v]
                    label_u    <= label_data;
                    label_addr <= vert_v[4:0];
                    state      <= S_GCD;
                end

                S_GCD: begin
                    // label[v] available, start GCD
                    label_v   <= label_data;
                    gcd_a     <= label_u;
                    gcd_b     <= label_data;
                    gcd_start <= 1;
                    state     <= S_CHECK;
                end

                S_CHECK: begin
                    if (gcd_done) begin
                        if (!gcd_coprime) begin
                            // Found a non-coprime edge
                            all_coprime <= 0;
                            fail_edge   <= edge_idx;
                            done        <= 1;
                            state       <= S_IDLE;
                        end else if (edge_idx + 1 >= num_edges) begin
                            // All edges checked, all coprime
                            all_coprime <= 1;
                            done        <= 1;
                            state       <= S_IDLE;
                        end else begin
                            // Next edge
                            edge_idx  <= edge_idx + 1;
                            edge_addr <= edge_idx + 1;
                            state     <= S_RD_EDGE;
                        end
                    end
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
