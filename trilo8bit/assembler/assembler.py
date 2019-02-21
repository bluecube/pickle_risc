import importlib.machinery
import importlib.util

from . import embeded
from .. import isa

class AssemblerError(Exception):
    pass

class Assembler:
    def __init__(self):
        self._data = []
        self._labels = {}

    def add_label(self, name):
        if name in self._labels:
            raise AssemblerError("Duplicate label name")
        self._labels[name] = len(self._instructions)

    @staticmethod
    def encode_number(n, size):
        """ Encode number `n` into `size` bytes. Little endian. """
        b = []
        for i in range(size):
            b.append((n >> (8 * i)) & 0xff)
        return bytes(b)

    def add_data(self, b):
        """ Add literal data to the output.
        All instructions end with this call. """
        if not isinstance(b, bytes):
            raise TypeError("Data to be added must be bytes")
        self._data.append(b)

    def load_file(self, filename):
        # Open asm the module
        loader = importlib.machinery.SourceFileLoader("_asm_input", filename)
        spec = importlib.util.spec_from_loader(loader.name, loader)
        asm_input = importlib.util.module_from_spec(spec)

        # Get the embeding context
        asm_embeding = embeded.Embeding(self)

        # Copy public data from the embeding into the loaded asm module
        for name in dir(asm_embeding):
            if not name.startswith("_") or name.isupper() or name == "_":
                setattr(asm_input, name, getattr(asm_embeding, name))

        # Execute it to collect the instructions
        spec.loader.exec_module(asm_input)
