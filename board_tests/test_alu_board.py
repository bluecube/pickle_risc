import pytest
import enum
import operator
import hypothesis
from hypothesis import strategies as st


class AluOpcode(enum.IntEnum):
    ADD = 0
    SHR1 = 1
    AND = 2
    OR = 3
    XOR = 4
    PACK = 5
    BCMP = 6
    OFF = 7


class AluBoard:
    @classmethod
    def make_sim(cls, verilog):
        b = cls()
        b.interface = verilog
        b.is_sim = True
        return b

    def make_tester(cls, tester):
        raise NotImplementedError("HW tester is not implemented yet")

    def __getitem__(self, key):
        return self.interface[key]

    def __setitem__(self, key, value):
        self.interface[key] = value

    def clock(self):
        """Tick the card clock"""
        self["clk"] = 1
        self["clk"] = 0

    def set_carry(self, carry):
        """Sets the carry flag using SHR1"""
        self["opcode"] = AluOpcode.SHR1
        self["immediate_for_b"] = 0
        self["b"] = int(carry)
        self.clock()
        assert self["carry"] == carry

    def reset_state(self, carry=0):
        """Reset the state of inputs and set carry to given state"""
        self.set_carry(carry)
        self["a"] = 0
        self["b"] = 0
        self["immediate"] = 0
        self["opcode"] = AluOpcode.OFF
        self["opcode_modifier"] = 0
        self["immediate_for_b"] = 0
        self["immediate_4bit"] = 0

    @staticmethod
    def sim_file_name():
        return "alu_board.sv"


BOARD_CLASS = AluBoard


def test_a_is_zero(board):
    """Check that a_is_zero is asserted correctly"""
    board["a"] = 0
    assert board["a_is_zero"] == 1


@pytest.mark.parametrize("i", range(16))
def test_a_is_nonzero_bit(board, i):
    """Check that a_is_zero when a single bit is set on A"""
    board["a"] = 1 << i
    assert board["a_is_zero"] == 0


def test_add_simple(board):
    """Test simple addition"""
    board.reset_state(carry=0)
    board["a"] = 10
    board["b"] = 20
    board["opcode"] = AluOpcode.ADD
    assert board["d"] == 30
    board.clock()
    assert board["carry"] == 0


def test_add_carry_out(board):
    """Test addition that should generate a carry"""
    board.reset_state(carry=0)
    board["a"] = 0xFFF0
    board["b"] = 0x0012
    board["opcode"] = AluOpcode.ADD
    board.clock()
    assert board["d"] == 2
    assert board["carry"] == 1


def test_add_with_carry(board):
    """Test addition with carry"""
    board.reset_state(carry=1)
    board["a"] = 10
    board["b"] = 20
    board["opcode"] = AluOpcode.ADD
    board["opcode_modifier"] = 2  # with carry
    assert board["d"] == 31
    board.clock()
    assert board["carry"] == 0


@hypothesis.given(
    a=st.integers(min_value=0, max_value=0xFFFF),
    b=st.integers(min_value=0, max_value=0xFFFF),
    c=st.integers(min_value=0, max_value=1)
)
def test_add(board, a, b, c):
    board.reset_state(carry=c)
    board["a"] = a
    board["b"] = b
    board["opcode"] = AluOpcode.ADD
    expected = a + b  # No carry!
    assert board["d"] == expected & 0xFFFF
    board.clock()
    assert board["carry"] == (expected >> 16) & 0x1


@hypothesis.given(
    a=st.integers(min_value=0, max_value=0xFFFF),
    b=st.integers(min_value=0, max_value=0xFFFF),
    c=st.integers(min_value=0, max_value=1)
)
def test_addc(board, a, b, c):
    board.reset_state(carry=c)
    board["a"] = a
    board["b"] = b
    board["opcode"] = AluOpcode.ADD
    board["opcode_modifier"] = 2  # with carry
    expected = a + b + c
    assert board["d"] == expected & 0xFFFF
    board.clock()
    assert board["carry"] == (expected >> 16) & 0x1


def test_sub_simple(board):
    """Test simple subtraction"""
    board.reset_state(carry=0)
    board["a"] = 30
    board["b"] = 20
    board["opcode"] = AluOpcode.ADD
    board["opcode_modifier"] = 1  # subtract
    assert board["d"] == 10
    board.clock()
    assert board["carry"] == 1  # For subtraction `carry` means "not borrow"


def test_sub_carry_out(board):
    """Test subtraction that should borrow"""
    board.reset_state(carry=0)
    board["a"] = 20
    board["b"] = 30
    board["opcode"] = AluOpcode.ADD
    board["opcode_modifier"] = 1  # subtract
    assert board["d"] == (-10 & 0xFFFF)
    board.clock()
    assert board["carry"] == 0  # For subtraction `carry` means "not borrow"


@hypothesis.given(
    a=st.integers(min_value=0, max_value=0xFFFF),
    b=st.integers(min_value=0, max_value=0xFFFF),
    c=st.integers(min_value=0, max_value=1)
)
def test_sub(board, a, b, c):
    board.reset_state(carry=c)
    board["a"] = a
    board["b"] = b
    board["opcode"] = AluOpcode.ADD
    board["opcode_modifier"] = 1  # subtract
    expected = a - b  # No carry!
    assert board["d"] == expected & 0xFFFF
    board.clock()
    assert board["carry"] == 1 - (expected >> 16) & 0x1


@hypothesis.given(
    a=st.integers(min_value=0, max_value=0xFFFF),
    b=st.integers(min_value=0, max_value=0xFFFF),
    c=st.integers(min_value=0, max_value=1)
)
def test_subc(board, a, b, c):
    board.reset_state(carry=c)
    board["a"] = a
    board["b"] = b
    board["opcode"] = AluOpcode.ADD
    board["opcode_modifier"] = 3  # subtract with carry
    expected = a - b + c - 1
    assert board["d"] == expected & 0xFFFF
    board.clock()
    assert board["carry"] == 1 - (expected >> 16) & 0x1


def test_shr_simple(board):
    board.reset_state(carry=0)
    board["opcode"] = AluOpcode.SHR1
    board["opcode_modifier"] = 0  # logical shift
    board["b"] = 0b1010_1010_1010_1010
    assert board["d"] == 0b0101_0101_0101_0101
    board.clock()
    assert board["carry"] == 0


def test_shr_carry_out(board):
    board.reset_state(carry=0)
    board["opcode"] = AluOpcode.SHR1
    board["opcode_modifier"] = 0  # logical shift
    board["b"] = 0b1010_1010_1010_1011
    assert board["d"] == 0b0101_0101_0101_0101
    board.clock()
    assert board["carry"] == 1


def test_shr_arithmetic(board):
    board.reset_state(carry=0)
    board["opcode"] = AluOpcode.SHR1
    board["opcode_modifier"] = 1  # arithmetic shift
    board["b"] = 0b1010_1010_1010_1010
    assert board["d"] == 0b1101_0101_0101_0101, f"d = {bin(board['b'])}"
    board.clock()
    assert board["carry"] == 0


@pytest.mark.parametrize("c", range(2))
def test_shr_carry(board, c):
    board.reset_state(carry=c)
    board["opcode"] = AluOpcode.SHR1
    board["opcode_modifier"] = 2  # shift with carry
    board["b"] = 0b1010_1010_1010_1010
    assert board["d"] == 0b101_0101_0101_0101 | c << 15
    board.clock()
    assert board["carry"] == 0


@hypothesis.given(
    a=st.integers(min_value=0, max_value=0xFFFF),
    b=st.integers(min_value=0, max_value=0xFFFF),
    c=st.integers(min_value=0, max_value=1)
)
@pytest.mark.parametrize("opcode_modifier", [
     pytest.param(0, id="logical"),
     pytest.param(1, id="arithmetic"),
     pytest.param(2, id="with_carry")
])
def test_shr(board, a, b, c, opcode_modifier):
    board.reset_state(carry=c)
    board["opcode"] = AluOpcode.SHR1
    board["opcode_modifier"] = opcode_modifier
    board["a"] = a  # A is ignored...
    board["b"] = b

    if opcode_modifier == 0:
        shifted_in_bit = 0  # Logical shift
    elif opcode_modifier == 1:
        shifted_in_bit = b >> 15  # Arithmetic shift
    else:
        shifted_in_bit = c  # Shift with carry

    assert board["d"] == b >> 1 | shifted_in_bit << 15
    board.clock()
    assert board["carry"] == b & 1


def test_and_simple(board):
    board.reset_state()
    board["a"] = 0xF0F0
    board["b"] = 0x0FF0
    board["opcode"] = AluOpcode.AND
    assert board["d"] == 0x00F0


def test_or_simple(board):
    board.reset_state()
    board["a"] = 0xF0F0
    board["b"] = 0x0FF0
    board["opcode"] = AluOpcode.OR
    assert board["d"] == 0xFFF0


def test_xor_simple(board):
    board.reset_state()
    board["a"] = 0xF0F0
    board["b"] = 0x0FF0
    board["opcode"] = AluOpcode.XOR
    assert board["d"] == 0xFF00


def test_pack_simple(board):
    board.reset_state()
    board["a"] = 0xABCD
    board["b"] = 0x1234
    board["opcode"] = AluOpcode.PACK
    board["opcode_modifier"] = 0  # pack low bytes
    assert board["d"] == 0x34CD


def test_bytes_swap_simple(board):
    board.reset_state()
    board["a"] = 0xABCD
    board["b"] = 0x1234
    board["opcode"] = AluOpcode.PACK
    board["opcode_modifier"] = 1  # byte swap
    assert board["d"] == 0x3412


def test_pass_a_simple(board):
    board.reset_state()
    board["a"] = 0xABCD
    board["b"] = 0x1234
    board["opcode"] = AluOpcode.PACK
    board["opcode_modifier"] = 2  # pass A
    assert board["d"] == 0xABCD


def test_pack_hi_simple(board):
    board.reset_state()
    board["a"] = 0xABCD
    board["b"] = 0x1234
    board["opcode"] = AluOpcode.PACK
    board["opcode_modifier"] = 2  # pass A
    assert board["d"] == 0xABCD


@pytest.mark.parametrize("a,expected", [
    pytest.param(0x1234, 0b00, id="00"),
    pytest.param(0x1278, 0b01, id="01"),
    pytest.param(0x7834, 0b10, id="10"),
    pytest.param(0x7878, 0b11, id="11")
])
def test_bcmp_simple(board, a, expected):
    board.reset_state()
    board["a"] = a
    board["b"] = 0x5678
    board["opcode"] = AluOpcode.BCMP
    assert board["d"] == expected


@hypothesis.given(
    a=st.integers(min_value=0, max_value=0xFFFF),
    b=st.integers(min_value=0, max_value=0xFFFF),
    c=st.integers(min_value=0, max_value=1),
)
@pytest.mark.parametrize("opcode,opcode_modifier,fun", [
    pytest.param(AluOpcode.AND, 0, operator.and_, id="and"),
    pytest.param(AluOpcode.OR, 0, operator.or_, id="or"),
    pytest.param(AluOpcode.XOR, 0, operator.xor, id="xor"),
    pytest.param(
        AluOpcode.PACK, 0,
        lambda a, b: (a & 0xFF) | ((b & 0xFF) << 8),
        id="pack"
    ),
    pytest.param(
        AluOpcode.PACK, 1,
        lambda a, b: ((b & 0xFF00) >> 8) | ((b & 0xFF) << 8),
        id="swap_bytes"
    ),
    pytest.param(AluOpcode.PACK, 2, lambda a, b: a, id="pass_a"),
    pytest.param(
        AluOpcode.PACK, 3,
        lambda a, b: (a & 0xFF00) | (b >> 8),
        id="pack_hi"
    ),
    pytest.param(
        AluOpcode.BCMP, 0,
        lambda a, b: int(a & 0xFF == b & 0xFF) | int(a >> 8 == b & 0xFF) << 1,
        id="bcmp"
    ),
])
def test_binary_no_carry(board, a, b, c, opcode, opcode_modifier, fun):
    board.reset_state(carry=c)
    board["a"] = a
    board["b"] = b
    board["opcode"] = opcode
    board["opcode_modifier"] = opcode_modifier
    assert board["d"] == fun(a, b)
    board.clock()
    assert board["carry"] == c


@pytest.mark.parametrize("immediate,immediate_4bit,expected", [
   pytest.param(0x1B, 0, 0x001B, id="8bit"),
   pytest.param(0xAB, 0, 0xFFAB, id="8bit_sign_extend"),
   pytest.param(0xA5, 1, 0x0005, id="4bit"),
   pytest.param(0x9A, 1, 0xFFFA, id="4bit_sign_extend"),
])
def test_immediate_simple(board, immediate, immediate_4bit, expected):
    board.reset_state()
    board["immediate_for_b"] = 1
    board["immediate"] = immediate
    board["immediate_4bit"] = immediate_4bit
    board["a"] = 0
    board["opcode"] = AluOpcode.OR
    assert board["d"] == expected


@pytest.mark.parametrize("immediate_4bit", [
   pytest.param(0, id="8bit"),
   pytest.param(1, id="4bit"),
])
def test_immediate_disabled(board, immediate_4bit):
    board.reset_state()
    board["immediate_for_b"] = 0
    board["immediate"] = 0xAA
    board["immediate_4bit"] = immediate_4bit
    board["a"] = 0
    board["b"] = 0x1234
    board["opcode"] = AluOpcode.OR
    assert board["d"] == 0x1234


@hypothesis.given(
    a=st.integers(min_value=0, max_value=0xFFFF),
    b=st.integers(min_value=0, max_value=0xFFFF),
    immediate=st.integers(min_value=0, max_value=0xFF),
)
@pytest.mark.parametrize("immediate_4bit", [
    pytest.param(0, id="8bit"), pytest.param(1, id="4bit")
])
def test_immediate(board, a, b, immediate, immediate_4bit):
    board.reset_state()
    board["immediate_for_b"] = 1
    board["immediate"] = immediate
    board["immediate_4bit"] = immediate_4bit
    board["a"] = 0
    board["opcode"] = AluOpcode.OR

    if immediate_4bit == 0:
        # 8-bit sign extension
        value_to_extend = immediate
        sign_bit = 1 << 7
    else:  # immediate_4bit == 1
        # 4-bit sign extension
        value_to_extend = immediate & 0xF
        sign_bit = 1 << 3
    expected = (
        (value_to_extend & (sign_bit - 1)) - (value_to_extend & sign_bit)
    ) & 0xFFFF

    assert board["d"] == expected


@pytest.mark.parametrize("c_in", range(2))
@pytest.mark.parametrize("c_out", range(2))
def test_carry_out_clocking(board, c_in, c_out):
    board.reset_state(carry=c_in)
    board["opcode"] = AluOpcode.SHR1
    board["opcode_modifier"] = 2  # With carry
    board["b"] = 0x0AA0 | c_out
    assert board["carry"] == c_in
    assert board["d"] == 0x0550 | c_in << 15

    board["clk"] = 1

    # Change the input so that it would result in different carry,
    # but the current one is already latched in
    board["b"] = 0xA00A | (1 - c_out)
    assert board["d"] == 0x5005 | c_in << 15  # D changes immediately
    assert board["carry"] == c_in  # No change in carry out until falling edge

    board["clk"] = 0

    assert board["carry"] == c_out # Change according to the previous input
    assert board["d"] == 0x5005 | c_out << 15  # The current carry output is used on D
