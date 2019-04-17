import contextlib

from . import assembler
from . import instructions

def function(decorated):
    """ Decorator for defining machine code functions.
    The function is replaced by the Function object, which supports methods
    inline() and call() from within another function and assemble() from the top level. """

    return assembler.Function(decorated)

def label(name):
    """ Create a label object.
    This is actually the only way to define forward label :-/ """

    return assembler.current_state.insert_symbol(name)

def raw_data(value):
    """ Write raw bytes data into the output stream, no labels. """
    assembler.current_state.insert_data(value)

def string_data(value, label=None, encoding="ascii"):
    """ Write bytes-like or string-like data into the output, optionally creating
    labels. """

    try:
        encoded = value.encode(encoding)
    except AttributeError:
        encoded = value

    label = assembler.current_state.insert_symbol(label)
    label.data = encoded
    assembler.current_state.insert_data(encoded)
    return label

def data(value, bits, label=None):
    """ Create a data object on the stack from an int or array of ints,
    return label to it. Big endian. """

    if bits % 8:
        raise ValueError("Bits must be divisible by 8")

    try:
        it = iter(value)
    except TypeError:
        processed_data = [value.to_bytes(bits // 8, "big")]
    else:
        processed_data = [x.to_bytes(bits // 8, "big") for x in value]

    label = assembler.current_state.insert_symbol(label)
    label.data = value
    assembler.current_state.insert_data(b"".join(processed_data))
    return label

def datafile(filename, label=None):
    with open(filename, "b") as fp:
        value = b.read()

    label = assembler.current_state.insert_symbol(label_name)
    assembler.current_state.insert_data(value)
    return label
