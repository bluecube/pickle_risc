.PHONY: all
all: instruction_set.html emulator assembler

instruction_set.html: instruction_set.json5 generate_instruction_set_html.py
	./generate_instruction_set_html.py -o $@ $<

.PHONY: emulator
emulator:
	${MAKE} -C emulator

.PHONY: assembler
assembler:
	${MAKE} -C assembler
