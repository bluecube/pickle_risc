def Register():
    """ Register that has a name """
    def __init__(self, name):
        self.name = name

    def __str__(self):
        return self.name

    def __repr__(self):
        return self.name

    def __pos__(self):
        """ Pre-decrement the register's value """
        return _OpRegister(self, "+")

    def __neg__(self):
        """ Post-increment the register's value """
        return _OpRegister(self, "-")

    def __not__(self):
        return _OpRegister(self, "not")

def _OpRegister:
    def __init__(self, reg, op):
        self.reg = reg
        self.op = op

    def __str__(self):
        return self.op + " " + str(self.reg)

    def __repr__(self):
        return str(self)
