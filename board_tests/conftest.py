import pytest
import pyverilator


def pytest_addoption(parser):
    parser.addoption(
        "--hw",
        action="store",
        default=None,
        help="TTY port for board tester (e.g. /dev/ttyUSB0). If not set, the tests run against the verilog models"
    )


@pytest.fixture(scope="module")
def board(request):
    hw_port = request.config.getoption("--hw")
    board_class = request.module.BOARD_CLASS

    if hw_port is None:
        test_dir = request.path.parent
        model_dir = test_dir.parent / "model"
        build_dir = test_dir / "verilator_build_dir"
        vcd_dir = test_dir / "vcd"
        model_file = model_dir / board_class.sim_file_name()
        vcd_file = (vcd_dir / board_class.sim_file_name()).with_suffix(".vcd")

        verilator = pyverilator.PyVerilator.build(
            model_file,
            build_dir=build_dir,
            cargs="--std=c++14"
        )
        verilator.clock = None  # Auto-tracing is based on eval instead of clock
        verilator.start_vcd_trace(str(vcd_file))

        yield board_class.make_sim(verilator)

        verilator.flush_vcd_trace()
        verilator.stop_vcd_trace()

    else:
        raise NotImplementedError("HW tester is not implemented yet")

        tester = board_class.make_tester(hw_port)
        yield board_class.make_tester(tester)
