typedef enum logic [2:0] {
  ADD = 3'd0,
  SHR1 = 3'd1,
  AND = 3'd2,
  OR = 3'd3,
  XOR = 3'd4,
  PACK = 3'd5, // Also includes SHR8 and pass B
  BCMP = 3'd6,
  OFF = 3'd7
} alu_opcode_t;

module alu_board (
  input logic clk,

  input logic [15:0] a,
  input logic [15:0] b,
  input logic [7:0] immediate,

  input alu_opcode_t opcode,
  input [1:0] opcode_modifier,
  input logic immediate_for_b,
  input logic immediate_4bit,

  output logic [15:0] d,
  output logic carry,
  output logic a_is_zero
);

  // Named wires for internal b input or immediate, and for resulting carry value
  logic internal_carry;
  logic updating_carry;

  // Latched version of carry for clocked writes
  logic carry_latched;

  always_ff @(posedge clk) begin
    if (updating_carry)
      carry_latched <= internal_carry;
  end
  
  always_ff @(negedge clk) begin
    carry <= carry_latched;
  end

  always_comb begin
    logic [15:0] bb; // BB is B or immediate
    if (immediate_for_b) begin
      if (immediate_4bit)
        bb = {{12{immediate[3]}}, immediate[3:0]};
      else
        bb = {{8{immediate[7]}}, immediate};
    end else
      bb = b;

    if (opcode == ADD) begin
      logic subtract = opcode_modifier[0];
      logic with_carry = opcode_modifier[1];
      logic modified_carry = with_carry ? carry : subtract;
      logic [15:0] bbb = subtract ? (~bb) : bb;

      {internal_carry, d} = a + bbb + modified_carry;
      updating_carry = 1;
    end else if (opcode == SHR1) begin
      logic arithmetic = opcode_modifier[0];
      logic with_carry = opcode_modifier[1];
      logic modified_carry; 
      if (with_carry)
        modified_carry = carry;
      else if (arithmetic)
        modified_carry = bb[15];
      else
        modified_carry = 0;

      d = {modified_carry, bb[15:1]};
      updating_carry = 1;
      internal_carry = bb[0];
    end else if (opcode == AND) begin
      d = a & bb;
      updating_carry = 0;
      internal_carry = 0;
    end else if (opcode == OR) begin
      d = a | bb;
      updating_carry = 0;
      internal_carry = 0;
    end else if (opcode == XOR) begin
      d = a ^ bb;
      updating_carry = 0;
      internal_carry = 0;
    end else if (opcode == PACK) begin
      // opcode_modifier -> function:
      // 2'b00 -> pack low bytes
      // 2'b01 -> bb byte swap
      // 2'b10 -> pass A
      // 2'b11 -> pack high bytes (shr8, if a == 0)
      if (opcode_modifier[1])
        d[15:8] = a[15:8];
      else
        d[15:8] = bb[7:0];
      if (opcode_modifier[0])
        d[7:0] = bb[15:8];
      else
        d[7:0] = a[7:0];
      updating_carry = 0;
      internal_carry = 0;
    end else if (opcode == BCMP) begin
      d = {
        14'd0,
        a[15:8] == bb[7:0],
        a[7:0] == bb[7:0]
      };
      updating_carry = 0;
      internal_carry = 0;
    end else begin
        d = 16'hxxxx;
        updating_carry = 0;
        internal_carry = 0;
    end
  end

  always_comb begin
    a_is_zero = (a == 0);
  end
endmodule
