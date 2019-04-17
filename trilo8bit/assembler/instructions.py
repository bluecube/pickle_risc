import string
import warnings
import collections
import itertools

from . import assembler
from .. import isa

class _OrderedCounter(collections.Counter, collections.OrderedDict):
    pass

def _parse_i(value, bits):
    """ Check range for immediate value and pass it through. Negative values work as well
    and are converted to two's complement. """
    maximum = (1 << bits) - 1
    signed_minimum = -(1 << (bits - 1))
    if signed_minimum <= value <= maximum:
        return value & maximum
    else:
        raise ValueError("Value out of range")

def _parse_j(value, bits):
    """ Parse relative jump argument and output.
    Value can either be a signed integer, in which case this function directly outputs it,
    or the value can be a string or label, in which case the jump distance is calculated
    as a difference between the current and target location. """

    maximum = (1 << (bits - 1)) - 1
    minimum = -maximum - 1
    try:
        if minimum <= value <= maximum:
            return value & ((1 << bits) - 1)
        else:
            raise ValueError("Value out of range")
    except TypeError:
        pass

    location = assembler.current_state.get_symbol_location(value)
    if location is None:
        return 0 # In first pass we just output zero everywhere

    value = location - assembler.current_state.get_current_location() - 1
    if minimum <= value <= maximum:
        return value & ((1 << bits) - 1)
    else:
        raise ValueError("Jump target too far")

def _generate():
    global _encodings
    _encodings = {}
    placeholder_found = False
    for mnemonic, (encoding, description) in isa.instructions.items():
        arg_bits = _OrderedCounter(encoding)
        placeholder_found = "_" in arg_bits or placeholder_found

        del arg_bits[' ']

        if sum(arg_bits.values()) % 8:
            raise ValueError(f"Non integer number of bytes found in encoding of instruction {mnemonic} ({encoding})")

        del arg_bits['0']
        del arg_bits['1']
        del arg_bits['_']

        ranges = []
        for char, bits in arg_bits.items():
            if char not in string.ascii_lowercase:
                raise ValueError(f"Unexpected character '{char}' found in encoding of instruction {mnemonic} ({encoding})")
            ranges.append(range(1 << bits))

        for arg_values in itertools.product(*ranges):
            print(mnemonic, arg_values)

        ordered_args = ", ".join(arg_bits)
        args_key = ", ".join(f"_parse_{char}({char}, {bits})" for char, bits in arg_bits.items())

        definition = f'''
def {mnemonic}({ordered_args}):
    """ {mnemonic} instruction auto generated based on ISA.
    {description} """

    key = ("{mnemonic}", {args_key})
    print("3", sorted(locals().keys()))
    try:
        encoded = encodings[key]
    except KeyError:
        assert key in isa.instructions
        raise ValueError("Invalid instruction argument")

    assembler.current_state.insert_data(encoded)
    '''
        #print(definition)
        exec(definition, globals(), locals())

        globals()[mnemonic] = locals()[mnemonic]

    if placeholder_found:
        warnings.warn("Placeholder found in encoding of instruction")

_generate()
