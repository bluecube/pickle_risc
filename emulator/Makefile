CFILES = $(wildcard *.c)
CFILES = $(wildcard devices/*.c)
COMMON_CFILES = $(wildcard ../common/*.c)
OFILES = $(addprefix build/,$(CFILES:.c=.o))
OFILES += $(subst ../common/,build/common/,$(COMMON_CFILES:.c=.o))
DEPFILES = $(OFILES:.o=.d)
PROG = build/pickle_emulator

CFLAGS += -Werror -Wall -Wextra -std=c99 -march=native
CFLAGS += -fsanitize=undefined -fsanitize=address
CFLAGS += -Og -g

LDFLAGS = $(CFLAGS)

DEPFLAGS = -MMD -MP -MF $(@:.o=.d)

OFILES += build/instruction_handler.o

$(PROG): $(OFILES)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^

clean:
	rm -rf build/

build/common/%.o: ../common/%.c
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) $(DEPFLAGS) -I. -c -o $@ $<

build/%.o: %.c
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) $(DEPFLAGS) -c -o $@ $<

build/%.o: build/%.c
	@mkdir -p $(@D)
	$(CC) $(CFLAGS) $(DEPFLAGS) -I. -c -o $@ $<

build/instruction_handler.c: ../instruction_set.json5
	@mkdir -p $(@D)
	./generate_instruction_handler.py --header-name "instruction_handler.h" -o $@ $<
build/instruction_handler.c: generate_instruction_handler.py

-include $(DEPFILES)

