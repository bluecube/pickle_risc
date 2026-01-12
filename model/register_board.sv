typedef enum logic [1:0] {
  A_SEL  = 2'b00,
  D_SEL  = 2'b01,
  A_ZERO = 2'b10,
  A_OFF  = 2'b11
} register_board_a_mode_t;

module register_board (
  input logic clk,
  input logic reset,

  input logic [15:0] d,
  input logic n_load,

  input register_board_a_mode_t a_mode,
  input logic n_b_en,

  input logic [3:0] d_sel,
  input logic [3:0] a_sel,
  input logic [3:0] b_sel,

  output logic [15:0] a,
  output logic [15:0] b
);

  logic [15:0] r0;
  logic [15:0] regs [1:15];

  // Latches for clocked writes
  logic [3:0] d_sel_latched;
  logic n_load_latched;
  logic [15:0] d_latched;

  function [15:0] reg_value(input [3:0] sel);
  begin
    if (sel == 0)
      reg_value = r0;
    else
      reg_value = regs[sel];
  end
  endfunction

  // Positive edge of CLK latches the values that are relevant to loading values
  always_ff @(posedge clk) begin
    d_sel_latched <= d_sel;
    n_load_latched <= n_load;
    d_latched <= d;
  end

  // Negative edge of clk actually writes the data to the registers
  // This will potentially cause the A and B outputs to change value.
  always_ff @(negedge clk) begin
    if (!n_load_latched && d_sel_latched != 4'd0) begin
      regs[d_sel_latched] <= d_latched;
    end
  end

  // Resetting the zero register
  // This is a workaround to allow using '574 for the zero register,
  // instead of adding a bus driver to the BOM. The zero register FFs get
  // zeroed when the CPU is reset, before they are undefined
  always_ff @(posedge reset) begin
    r0 <= 16'd0;
  end

  // Driving the A output
  always_comb begin
    // For some reason the following case statement was not working...
    // case (a_mode)
    //   A_SEL: a = reg_value(a_sel);
    //   D_SEL: a = reg_value(d_sel);
    //   A_ZERO: a = r0;
    //   A_OFF: a = 16'hz;
    //   default: a = 16'hx;
    // endcase
    if (a_mode == A_SEL)
      a = reg_value(a_sel);
    else if (a_mode == D_SEL)
      a = reg_value(d_sel);
    else if (a_mode == A_ZERO)
      a = r0;
    else if (a_mode == A_OFF)
      a = 16'hz;
    else
      a = 16'hx;
  end

  // Driving the B output
  always_comb begin
    if (n_b_en)
      b = 16'hz;
    else
      b = reg_value(b_sel);
  end
endmodule
