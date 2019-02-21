from . import assembler

import sys

asm = assembler.Assembler()
asm.load_file(sys.argv[1])
