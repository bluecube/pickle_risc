from .. import isa

class AssemblerError(Exception):
    pass

class _AssemblerState:
    """ Assembler state shared between both passes. """

    def __init__(self):
        self._functions_to_link_list = []
        self._functions_to_link_set = set()

    def add_function(self, function):
        """ Add a function to a list functions to link if it hasn't been added before. """
        if function not in self._functions_to_link_set:
            self._functions_to_link_list.append(function)
            self._functions_to_link_set.add(function)

    def functions_to_link(self):
        """ Iterate over not yet processed functions. Adding new functions while iterating is ok. """
        i = 0
        while i < len(self._functions_to_link_list):
            yield self._functions_to_link_list[i]
            i += 1


class _Pass1State(_AssemblerState):
    def __init__(self, offset = 0):
        super().__init__()
        self._offset = offset
        self._symbol_table = {}

        self._location = self._offset

    def insert_data(self, raw_data):
        self._location += len(raw_data)

    def get_current_location(self):
        return self._location

    def get_symbol_location(self, symbol):
        return None

    def insert_symbol(self, name):
        sym = Symbol(name, self._location)

        try:
            previous = self._symbol_table[name]
        except KeyError:
            pass
        else:
            raise AssemblerError("Redefining symbol (new: " + str(sym) + ", previous: " + str(previous) + ")")

        self._symbol_table[name] = sym

        return sym

    def create_generate_top_level2_state(self):
        """ Create a state for second pass from this. """
        return _Pass2State(self._offset, self._symbol_table)


class _Pass2State(_AssemblerState):
    def __init__(self, offset, symbol_table):
        super().__init__()
        self._offset = offset
        self._symbol_table = symbol_table

        self.data = bytearray()

    def insert_data(self, raw_data):
        self.data.extend(raw_data)

    def get_current_location(self):
        return len(self.data) + self._offset

    def get_symbol_location(self, symbol):
        if isinstance(symbol, Symbol):
            return symbol.location
        else:
            try:
                return self._symbol_table[name].location
            except KeyError:
                raise AssemblerError(f"Symbol {name} not found")

    def insert_symbol(self, name):
        sym = Symbol(name, len(self.data) + self._offset)

        try:
            previous = self._symbol_table[name]
        except KeyError:
            raise AssemblerError(f"Symbol not defined from first pass ({sym})")

        if previous != sym:
            raise AssemblerError(f"Symbol changed since first pass (new: {sym}, previous: {previous})")

        return previous


class Function:
    def __init__(self, fun):
        self._fun = fun
        self.name = fun.__name__
        self.__doc__ = fun.__doc__

    def call(self):
        """ Generate the code to call this function, mark it for linking.
        Calling a function clobbers the A register. """

        instructions.load_immediate_a_lo(lo8(self.name))
        instructions.load_immediate_a_hi(hi8(self.name))
        instructions.swap_pc()

        current_state.add_function(self)

    def inline(self):
        """ Output the function's code directly at the call site. """
        with current_state.scoped_state(self.name):
            self._fun()

    def assemble(self):
        """ Assemble the function and all other functions called from it.
        Returns a bytes object with the output. """

        global current_state

        if current_state is not None:
            raise AssemblerError("Don't call assemble() from other functions!")

        current_state = _Pass1State(0)
        self._generate_top_level()
        current_state = current_state.create_generate_top_level2_state()
        self._generate_top_level()

        ret = current_state.data

        current_state = None

        return ret

    def _generate_top_level(self):
        current_state.insert_symbol(self.name) # Support endless loops in top level function
        self._fun()
        for function in current_state.functions_to_link():
            function._generate_callee()

    def _generate_callee(self):
        """ Output code of the function that can be called, define label for the function. """
        current_state.insert_symbol(self.name)
        with current_state.scoped_state(self.name):
            self._fun()


class Symbol:
    """ Symbol representing a location in the output stream """
    def __init__(self, name, location):
        self.name = name
        self.location = location

    def __int__(self):
        return self.location

    def __str__(self):
        return f"{self.name} ({_format_address(self.location)})"

    def __eq__(self, other):
        return self.name == other.name and self.location == other.location


def _format_address(address):
    return f"0x{address:04x}"

current_state = None
