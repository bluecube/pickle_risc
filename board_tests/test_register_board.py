import pytest
import enum
import hypothesis


class AMode(enum.IntEnum):
    A_SEL = 0b00
    D_SEL = 0b01
    A_ZERO = 0b10
    A_OFF = 0b11


class RegisterBoard:
    @classmethod
    def make_sim(cls, verilog):
        b = cls()
        b.interface = verilog
        b.is_sim = True
        return b

    def make_tester(cls, tester):
        raise NotImplementedError("HW tester is not implemented yet")

    def reset(self):
        self["reset"] = 1
        self["reset"] = 0

    def read_a(self, reg):
        self["a_sel"] = reg
        self["a_mode"] = AMode.A_SEL
        val = self["a"]
        self["a_mode"] = AMode.A_OFF
        return val

    def read_b(self, reg):
        self["b_sel"] = reg
        self["n_b_en"] = 0
        val = self["b"]
        self["n_b_en"] = 1
        return val

    def read_d(self, reg):
        self["d_sel"] = reg
        self["a_mode"] = AMode.D_SEL
        val = self["a"]
        return val

    def write(self, reg, value):
        self["n_load"] = 0
        self["d_sel"] = reg
        self["d"] = value
        self["clk"] = 1
        self["clk"] = 0
        self["n_load"] = 1

    def __getitem__(self, key):
        return self.interface[key]

    def __setitem__(self, key, value):
        self.interface[key] = value

    @staticmethod
    def sim_file_name():
        return "register_board.sv"


BOARD_CLASS = RegisterBoard


def test_r1_write_and_read(board):
    """Simple write and read back from r1"""
    board.write(1, 0x1234)

    board["d"] = 0x5678  # Changing value of D should not change the value

    assert board.read_a(1) == 0x1234


def test_r0_after_reset_a(board):
    """Check that r0 reads as zero after the board is reset"""
    board.reset()
    assert board.read_a(0) == 0
    assert board.read_b(0) == 0
    assert board.read_d(0) == 0


def test_r0_is_immutable(board):
    """Verify that writing to r0 is ignored."""
    board.write(0, 0xffff)
    assert board.read_a(0) == 0


def test_a_mode_mux(board):
    """Verifies all four modes of the A output mux."""
    board.write(2, 0xabcd)
    board.write(4, 0x1234)

    # Mode: A_ZERO
    board["a_mode"] = AMode.A_ZERO
    assert board["a"] == 0

    assert board.read_d(2) == 0xabcd  # Read using D_SEL
    assert board.read_a(4) == 0x1234  # Read using A_SEL

    # In simulation, 'z' often holds the previous value or 0/MAX
    # depending on wrapper, but shouldn't change if b_sel changes now
    # TODO: What to do with tester?
    board["a_mode"] = AMode.A_OFF
    val_disabled = board["a"]
    board["a_sel"] = 2
    board["d_sel"] = 4
    assert board["a"] == val_disabled  # should not change because of sel change


def test_port_b_tristate(board):
    """Verify that Port B only drives when n_b_en is 0."""
    board.write(7, 0x7777)
    board.write(8, 0x8888)

    assert board.read_b(7) == 0x7777
    board["n_b_en"] = 1
    val_disabled = board["b"]

    # In simulation, 'z' often holds the previous value or 0/MAX
    # depending on wrapper, but shouldn't change if b_sel changes now
    # TODO: What to do with tester?
    board["b_sel"] = 8
    assert board["b"] == val_disabled  # should not change because of sel change


@pytest.mark.parametrize("name, values", [
    ("Zeros", [0x0000]*16),
    ("Ones",  [0xFFFF]*16),
    ("Checkerboard", [0xAAAA if i % 2 == 0 else 0x5555 for i in range(16)]),
    ("InverseCheckerboard", [0x5555 if i % 2 == 0 else 0xAAAA for i in range(16)]),
    ("RegID", [i for i in range(16)]),
    ("InverseRegID", [0xffff - i for i in range(16)]),
])
def test_integrity_patterns(board, name, values):
    """Writes pattern and verifies every register through Port A and B."""
    for i, val in enumerate(values):
        board.write(i, val)

    for i, val in enumerate(values):
        expected = 0 if i == 0 else val

        assert board.read_a(i) == expected
        assert board.read_b(i) == expected
        assert board.read_d(i) == expected


def test_write_latched_inputs(board):
    """
    Verify that data/address are sampled on POSEDGE but written on NEGEDGE.
    If we change inputs between edges, the OLD values should be written.
    """
    # Clear the registers so that we don't have stale state messing with the test
    board.write(5, 0)
    board.write(8, 0)

    # Setup initial values at clk rising edge
    board["n_load"] = 0
    board["d_sel"] = 5
    board["d"] = 0xaaaa
    board["clk"] = 1

    # change inputs
    board["d_sel"] = 8
    board["d"] = 0x5555

    # clk falling edge
    board["clk"] = 0

    assert board.read_a(5) == 0xaaaa
    assert board.read_a(8) == 0


def test_address_decoding_isolation(board):
    """Ensure writing to one register doesn't corrupt others."""
    for i in range(1, 16):
        board.write(i, 0x100 * i)

    for i in range(16):
        board.write(i, 0xffff)
        for j in range(15):
            if j >= i:
                j += 1
            assert board.read_a(j) == 0x100 * j
        board.write(i, 0x100 * i)


def test_no_write_without_n_load(board):
    """Verify no write occurs if n_load is high during clock pulse."""
    board.write(3, 0xcafe)
    board["n_load"] = 1
    board["d_sel"] = 3
    board["d"] = 0xdead
    board["clk"] = 1
    board["clk"] = 0

    assert board.read_a(3) == 0xcafe


def test_no_write_without_clock(board):
    """Verify no write occurs just by setting n_load low without clock."""
    board.write(3, 0xcafe)
    board["d_sel"] = 3
    board["d"] = 0xdead
    board["n_load"] = 0
    board["n_load"] = 1

    assert board.read_a(3) == 0xcafe


@hypothesis.given(
    hypothesis.strategies.lists(
        hypothesis.strategies.one_of(
            hypothesis.strategies.tuples(
                hypothesis.strategies.sampled_from(["read_a", "read_b", "read_d"]),
                hypothesis.strategies.integers(min_value=0, max_value=15),
            ),
            hypothesis.strategies.tuples(
                hypothesis.strategies.just("write"),
                hypothesis.strategies.integers(min_value=0, max_value=15),
                hypothesis.strategies.integers(min_value=0, max_value=0xFFFF),
            )
        ),
        min_size=1,
        max_size=200,
    )
)
def test_randomized_operations(board, ops):
    expected = [0] * 16
    board.reset()
    for i in range(1, 16):
        board.write(i, expected[i])

    for op in ops:
        if op[0] == "write":
            _, reg, val = op
            board.write(reg, val)
            if reg != 0:
                expected[reg] = val

        else:
            _, reg = op
            if op[0] == "read_a":
                val = board.read_a(reg)
            elif op[0] == "read_b":
                val = board.read_b(reg)
            else:
                val = board.read_d(reg)

            assert val == expected[reg]

    for i in range(16):
        assert board.read_a(i) == expected[i]
