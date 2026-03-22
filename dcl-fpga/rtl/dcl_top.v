`timescale 1ns / 1ps
// DCL Top Module — Nexys 4 DDR (XC7A100T-1CSG324C)
// Integrates: GCD, MulMod, PowerMap, CoprimeChecker, UART, Demo, 7-Seg

module dcl_top (
    input  wire        CLK100MHZ,
    input  wire        CPU_RESETN,    // active-low reset

    // UART
    input  wire        UART_TXD_IN,   // PC → FPGA (RX)
    output wire        UART_RXD_OUT,  // FPGA → PC (TX)

    // Switches and buttons
    input  wire [15:0] SW,
    input  wire        BTNC, BTNU, BTND, BTNL, BTNR,

    // LEDs
    output wire [15:0] LED,
    output wire        LED16_R, LED16_G, LED16_B,

    // 7-segment display
    output wire [6:0]  SEG,           // CA..CG (active low)
    output wire        DP,
    output wire [7:0]  AN
);

    wire clk   = CLK100MHZ;
    wire rst_n = CPU_RESETN;

    // ---- UART ----
    wire [7:0] rx_data;
    wire       rx_valid_raw;
    wire [7:0] tx_data;
    wire       tx_send;
    wire       tx_busy;

    uart_rx u_uart_rx (
        .clk   (clk),
        .rst_n (rst_n),
        .rx    (UART_TXD_IN),
        .data  (rx_data),
        .valid (rx_valid_raw)
    );

    uart_tx u_uart_tx (
        .clk   (clk),
        .rst_n (rst_n),
        .data  (tx_data),
        .send  (tx_send),
        .tx    (UART_RXD_OUT),
        .busy  (tx_busy)
    );

    // Gate UART rx_valid with !SW[15] — UART is disabled in demo mode
    wire rx_valid = rx_valid_raw && !SW[15];

    // ---- demo_active signal (driven by demo_ctrl.active) ----
    wire demo_active;

    // ---- GCD Unit — mux inputs from cmd_dispatch or demo_ctrl ----
    wire [63:0] gcd_a_cmd,    gcd_b_cmd;
    wire        gcd_start_cmd;
    wire [63:0] gcd_a_demo,   gcd_b_demo;
    wire        gcd_start_demo;

    wire [63:0] gcd_result;
    wire        gcd_done, gcd_coprime;

    wire [63:0] gcd_a_mux     = demo_active ? gcd_a_demo     : gcd_a_cmd;
    wire [63:0] gcd_b_mux     = demo_active ? gcd_b_demo     : gcd_b_cmd;
    wire        gcd_start_mux = demo_active ? gcd_start_demo : gcd_start_cmd;

    gcd_unit u_gcd (
        .clk       (clk),
        .rst_n     (rst_n),
        .start     (gcd_start_mux),
        .a         (gcd_a_mux),
        .b         (gcd_b_mux),
        .result    (gcd_result),
        .done      (gcd_done),
        .is_coprime(gcd_coprime)
    );

    // ---- Power Map Unit (contains its own mulmod_64 instance) ----
    wire [63:0] pm_x_cmd,    pm_x_demo;
    wire [31:0] pm_m_cmd,    pm_m_demo;
    wire [63:0] pm_mod_cmd,  pm_mod_demo;
    wire        pm_start_cmd, pm_start_demo;

    wire [63:0] pm_result;
    wire        pm_done;

    wire [63:0] pm_x_mux      = demo_active ? pm_x_demo    : pm_x_cmd;
    wire [31:0] pm_m_mux      = demo_active ? pm_m_demo    : pm_m_cmd;
    wire [63:0] pm_mod_mux    = demo_active ? pm_mod_demo  : pm_mod_cmd;
    wire        pm_start_mux  = demo_active ? pm_start_demo : pm_start_cmd;

    power_map_unit u_power_map (
        .clk    (clk),
        .rst_n  (rst_n),
        .start  (pm_start_mux),
        .x      (pm_x_mux),
        .m      (pm_m_mux),
        .modulus(pm_mod_mux),
        .result (pm_result),
        .done   (pm_done)
    );

    // ---- Label BRAM: 32 × 64-bit ----
    // Write port: from cmd_dispatch
    // Read port:  from coprime_checker
    wire [4:0]  label_waddr;
    wire [63:0] label_wdata;
    wire        label_we;
    wire [4:0]  label_raddr;   // driven by coprime_checker
    reg  [63:0] label_rdata;

    reg [63:0] label_mem [0:31];

    always @(posedge clk) begin
        if (label_we)
            label_mem[label_waddr] <= label_wdata;
        label_rdata <= label_mem[label_raddr];
    end

    // ---- Edge BRAM: 256 × (u8, u8) ----
    // Write port: from cmd_dispatch
    // Read port:  from coprime_checker
    wire [7:0] edge_waddr;
    wire [7:0] edge_u_wdata, edge_v_wdata;
    wire       edge_we;
    wire [7:0] edge_raddr;     // driven by coprime_checker
    reg  [7:0] edge_u_rdata, edge_v_rdata;

    reg [7:0] edge_u_mem [0:255];
    reg [7:0] edge_v_mem [0:255];

    always @(posedge clk) begin
        if (edge_we) begin
            edge_u_mem[edge_waddr] <= edge_u_wdata;
            edge_v_mem[edge_waddr] <= edge_v_wdata;
        end
        edge_u_rdata <= edge_u_mem[edge_raddr];
        edge_v_rdata <= edge_v_mem[edge_raddr];
    end

    // ---- Coprime Checker ----
    wire [7:0] cc_num_edges;
    wire       cc_start;
    wire       cc_all_coprime, cc_done;
    wire [7:0] cc_fail_edge;

    coprime_checker u_coprime (
        .clk         (clk),
        .rst_n       (rst_n),
        .start       (cc_start),
        .num_edges   (cc_num_edges),
        .label_data  (label_rdata),
        .label_addr  (label_raddr),
        .edge_u_data (edge_u_rdata),
        .edge_v_data (edge_v_rdata),
        .edge_addr   (edge_raddr),
        .all_coprime (cc_all_coprime),
        .done        (cc_done),
        .fail_edge   (cc_fail_edge)
    );

    // ---- Command Dispatcher ----
    wire        cmd_busy;
    wire [7:0]  cmd_n_labels, cmd_n_edges;

    cmd_dispatch u_cmd (
        .clk              (clk),
        .rst_n            (rst_n),
        // UART
        .rx_data          (rx_data),
        .rx_valid         (rx_valid),
        .tx_data          (tx_data),
        .tx_send          (tx_send),
        .tx_busy          (tx_busy),
        // GCD
        .gcd_a            (gcd_a_cmd),
        .gcd_b            (gcd_b_cmd),
        .gcd_start        (gcd_start_cmd),
        .gcd_result       (gcd_result),
        .gcd_done         (gcd_done),
        .gcd_coprime      (gcd_coprime),
        // Power Map
        .pm_x             (pm_x_cmd),
        .pm_m             (pm_m_cmd),
        .pm_modulus       (pm_mod_cmd),
        .pm_start         (pm_start_cmd),
        .pm_result        (pm_result),
        .pm_done          (pm_done),
        // Coprime Checker
        .cc_num_edges     (cc_num_edges),
        .cc_start         (cc_start),
        .cc_all_coprime   (cc_all_coprime),
        .cc_done          (cc_done),
        .cc_fail_edge     (cc_fail_edge),
        // BRAM write — labels
        .bram_label_waddr (label_waddr),
        .bram_label_wdata (label_wdata),
        .bram_label_we    (label_we),
        // BRAM write — edges
        .bram_edge_waddr  (edge_waddr),
        .bram_edge_u_wdata(edge_u_wdata),
        .bram_edge_v_wdata(edge_v_wdata),
        .bram_edge_we     (edge_we),
        // Status
        .n_labels         (cmd_n_labels),
        .n_edges          (cmd_n_edges),
        .busy             (cmd_busy)
    );

    // ---- Demo Controller ----
    wire [31:0] demo_display;
    wire [7:0]  demo_enable;
    wire [15:0] demo_led;
    wire [2:0]  demo_rgb;    // {R, G, B} from demo_ctrl.led16_rgb

    demo_ctrl u_demo (
        .clk           (clk),
        .rst_n         (rst_n),
        .demo_mode     (SW[15]),
        .sw            (SW[14:0]),
        .btnc          (BTNC),
        .btnu          (BTNU),
        .btnd          (BTND),
        // GCD
        .gcd_a         (gcd_a_demo),
        .gcd_b         (gcd_b_demo),
        .gcd_start     (gcd_start_demo),
        .gcd_result    (gcd_result),
        .gcd_done      (gcd_done),
        .gcd_coprime   (gcd_coprime),
        // Power Map
        .pm_x          (pm_x_demo),
        .pm_m          (pm_m_demo),
        .pm_modulus    (pm_mod_demo),
        .pm_start      (pm_start_demo),
        .pm_result     (pm_result),
        .pm_done       (pm_done),
        // Display
        .display_value (demo_display),
        .display_enable(demo_enable),
        .led           (demo_led),
        .led16_rgb     (demo_rgb),
        // State
        .active        (demo_active)
    );

    // ---- 7-Segment Display ----
    // Show demo display value in demo mode, blank otherwise
    wire [31:0] seg_value  = demo_active ? demo_display : 32'd0;
    wire [7:0]  seg_enable = demo_active ? demo_enable  : 8'h00;

    seven_seg u_seven_seg (
        .clk   (clk),
        .rst_n (rst_n),
        .value (seg_value),
        .enable(seg_enable),
        .seg   (SEG),
        .dp    (DP),
        .an    (AN)
    );

    // ---- LED Outputs ----
    // demo_active: show demo LEDs and RGB
    // normal mode: LED[7:0] = n_labels, LED[15] = cmd_busy, others 0
    assign LED    = demo_active ? demo_led
                                : {cmd_busy, 7'd0, cmd_n_labels};

    // demo_rgb[2]=R, demo_rgb[1]=G, demo_rgb[0]=B
    assign LED16_R = demo_active ? demo_rgb[2] : cmd_busy;
    assign LED16_G = demo_active ? demo_rgb[1] : !cmd_busy;
    assign LED16_B = demo_active ? demo_rgb[0] : 1'b0;

endmodule
