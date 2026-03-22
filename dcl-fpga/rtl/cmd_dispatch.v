`timescale 1ns / 1ps
// Command Dispatcher — parses UART commands [CMD][LEN][PAYLOAD]
// Routes to compute units and sends responses via UART TX
// Protocol defined in design doc: 7 commands (0x01–0x07)

module cmd_dispatch (
    input  wire        clk,
    input  wire        rst_n,

    // UART RX interface
    input  wire [7:0]  rx_data,
    input  wire        rx_valid,

    // UART TX interface
    output reg  [7:0]  tx_data,
    output reg         tx_send,
    input  wire        tx_busy,

    // GCD unit interface
    output reg  [63:0] gcd_a, gcd_b,
    output reg         gcd_start,
    input  wire [63:0] gcd_result,
    input  wire        gcd_done,
    input  wire        gcd_coprime,

    // Power Map unit interface
    output reg  [63:0] pm_x,
    output reg  [31:0] pm_m,
    output reg  [63:0] pm_modulus,
    output reg         pm_start,
    input  wire [63:0] pm_result,
    input  wire        pm_done,

    // Coprime checker interface
    output reg  [7:0]  cc_num_edges,
    output reg         cc_start,
    input  wire        cc_all_coprime,
    input  wire        cc_done,
    input  wire [7:0]  cc_fail_edge,

    // BRAM write ports (for STORE_LABEL, STORE_EDGE)
    output reg  [4:0]  bram_label_waddr,
    output reg  [63:0] bram_label_wdata,
    output reg         bram_label_we,

    output reg  [7:0]  bram_edge_waddr,
    output reg  [7:0]  bram_edge_u_wdata,
    output reg  [7:0]  bram_edge_v_wdata,
    output reg         bram_edge_we,

    // Status
    output reg  [7:0]  n_labels,
    output reg  [7:0]  n_edges,

    // Busy indicator
    output reg         busy
);

    // Commands
    localparam CMD_GCD           = 8'h01,
               CMD_POWER_MAP    = 8'h02,
               CMD_STORE_LABEL  = 8'h03,
               CMD_STORE_EDGE   = 8'h04,
               CMD_CHECK_COPRIME = 8'h05,
               CMD_EVOLVE       = 8'h06,
               CMD_STATUS       = 8'h07;

    // S_CMD removed (was declared but never entered).
    // S_TX_DELAY added to absorb the 1-cycle latency before tx_busy asserts.
    localparam S_IDLE      = 4'd0,
               S_LEN       = 4'd2,
               S_PAYLOAD   = 4'd3,
               S_EXECUTE   = 4'd4,
               S_WAIT      = 4'd5,
               S_RESPOND   = 4'd6,
               S_TX_WAIT   = 4'd7,
               S_TX_DELAY  = 4'd8;

    reg [3:0]  state;
    reg [7:0]  cmd;
    reg [7:0]  payload_len;
    reg [7:0]  payload [0:31];  // max 32 bytes payload buffer
    reg [7:0]  payload_idx;

    // Response buffer
    reg [7:0]  resp [0:31];
    reg [7:0]  resp_len;
    reg [7:0]  resp_idx;

    integer i;

    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state        <= S_IDLE;
            busy         <= 0;
            tx_send      <= 0;
            tx_data      <= 0;
            gcd_start    <= 0;
            pm_start     <= 0;
            cc_start     <= 0;
            bram_label_we <= 0;
            bram_edge_we  <= 0;
            n_labels     <= 0;
            n_edges      <= 0;
            cmd          <= 0;
            payload_len  <= 0;
            payload_idx  <= 0;
            resp_len     <= 0;
            resp_idx     <= 0;
            for (i = 0; i < 32; i = i + 1) begin
                payload[i] <= 0;
                resp[i]    <= 0;
            end
        end else begin
            tx_send       <= 0;
            gcd_start     <= 0;
            pm_start      <= 0;
            cc_start      <= 0;
            bram_label_we <= 0;
            bram_edge_we  <= 0;

            case (state)
                S_IDLE: begin
                    busy <= 0;
                    if (rx_valid) begin
                        cmd   <= rx_data;
                        state <= S_LEN;
                    end
                end

                S_LEN: begin
                    if (rx_valid) begin
                        payload_len <= rx_data;
                        payload_idx <= 0;
                        if (rx_data == 0)
                            state <= S_EXECUTE;
                        else
                            state <= S_PAYLOAD;
                    end
                end

                S_PAYLOAD: begin
                    if (rx_valid) begin
                        if (payload_idx < 32)
                            payload[payload_idx] <= rx_data;
                        if (payload_idx + 1 >= payload_len)
                            state <= S_EXECUTE;
                        payload_idx <= payload_idx + 1;
                    end
                end

                S_EXECUTE: begin
                    busy <= 1;
                    case (cmd)
                        CMD_GCD: begin
                            // Payload: a[8B] b[8B] (little-endian)
                            gcd_a <= {payload[7], payload[6], payload[5], payload[4],
                                      payload[3], payload[2], payload[1], payload[0]};
                            gcd_b <= {payload[15], payload[14], payload[13], payload[12],
                                      payload[11], payload[10], payload[9], payload[8]};
                            gcd_start <= 1;
                            state <= S_WAIT;
                        end

                        CMD_POWER_MAP: begin
                            // Payload: x[8B] m[4B] mod[8B]
                            pm_x <= {payload[7], payload[6], payload[5], payload[4],
                                     payload[3], payload[2], payload[1], payload[0]};
                            pm_m <= {payload[11], payload[10], payload[9], payload[8]};
                            pm_modulus <= {payload[19], payload[18], payload[17], payload[16],
                                           payload[15], payload[14], payload[13], payload[12]};
                            pm_start <= 1;
                            state <= S_WAIT;
                        end

                        CMD_STORE_LABEL: begin
                            // Payload: idx[1B] label[8B]
                            bram_label_waddr <= payload[0][4:0];
                            bram_label_wdata <= {payload[8], payload[7], payload[6], payload[5],
                                                  payload[4], payload[3], payload[2], payload[1]};
                            bram_label_we <= 1;
                            if (payload[0] >= n_labels)
                                n_labels <= payload[0] + 1;
                            // ACK
                            resp[0] <= 8'h01;
                            resp_len <= 1;
                            resp_idx <= 0;
                            state <= S_RESPOND;
                        end

                        CMD_STORE_EDGE: begin
                            // Payload: idx[1B] u[1B] v[1B]
                            bram_edge_waddr   <= payload[0];
                            bram_edge_u_wdata <= payload[1];
                            bram_edge_v_wdata <= payload[2];
                            bram_edge_we <= 1;
                            if (payload[0] >= n_edges)
                                n_edges <= payload[0] + 1;
                            resp[0] <= 8'h01;
                            resp_len <= 1;
                            resp_idx <= 0;
                            state <= S_RESPOND;
                        end

                        CMD_CHECK_COPRIME: begin
                            // Payload: num_edges[1B]
                            cc_num_edges <= payload[0];
                            cc_start <= 1;
                            state <= S_WAIT;
                        end

                        CMD_STATUS: begin
                            // No payload
                            resp[0] <= n_labels;
                            resp[1] <= n_edges;
                            resp_len <= 2;
                            resp_idx <= 0;
                            state <= S_RESPOND;
                        end

                        default: begin
                            // Unknown command — ignore
                            state <= S_IDLE;
                        end
                    endcase
                end

                S_WAIT: begin
                    // Wait for compute unit to finish
                    case (cmd)
                        CMD_GCD: begin
                            if (gcd_done) begin
                                // Response: gcd[8B] coprime[1B]
                                {resp[7], resp[6], resp[5], resp[4],
                                 resp[3], resp[2], resp[1], resp[0]} <= gcd_result;
                                resp[8] <= {7'd0, gcd_coprime};
                                resp_len <= 9;
                                resp_idx <= 0;
                                state <= S_RESPOND;
                            end
                        end

                        CMD_POWER_MAP: begin
                            if (pm_done) begin
                                {resp[7], resp[6], resp[5], resp[4],
                                 resp[3], resp[2], resp[1], resp[0]} <= pm_result;
                                resp_len <= 8;
                                resp_idx <= 0;
                                state <= S_RESPOND;
                            end
                        end

                        CMD_CHECK_COPRIME: begin
                            if (cc_done) begin
                                resp[0] <= {7'd0, cc_all_coprime};
                                resp[1] <= cc_fail_edge;
                                resp_len <= 2;
                                resp_idx <= 0;
                                state <= S_RESPOND;
                            end
                        end

                        default: state <= S_IDLE;
                    endcase
                end

                S_RESPOND: begin
                    // Wait until TX is free, then send the current response byte.
                    // After asserting tx_send we go to S_TX_DELAY for 1 cycle so
                    // that uart_tx has time to register the send and assert tx_busy
                    // before we inspect it in S_TX_WAIT.
                    if (!tx_busy) begin
                        tx_data <= resp[resp_idx];
                        tx_send <= 1;
                        state   <= S_TX_DELAY;
                    end
                end

                S_TX_DELAY: begin
                    // One pipeline bubble: tx_busy will be high on the next cycle.
                    state <= S_TX_WAIT;
                end

                S_TX_WAIT: begin
                    // Wait for uart_tx to finish transmitting the current byte.
                    if (!tx_busy) begin
                        if (resp_idx + 1 >= resp_len) begin
                            // All bytes sent — return to idle.
                            state <= S_IDLE;
                        end else begin
                            // Advance to the next byte.
                            resp_idx <= resp_idx + 1;
                            state    <= S_RESPOND;
                        end
                    end
                end
            endcase
        end
    end
endmodule
