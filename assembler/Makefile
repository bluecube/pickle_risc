CFILES = $(wildcard *.c)
OFILES = $(addprefix build/,$(CFILES:.c=.o))
DEPFILES = $(OFILES:.o=.d)
PROG = build/pickle_as

CFLAGS += -Wall -Wextra -std=c99 -D_ISOC99_SOURCE -march=native
CFLAGS += -fsanitize=undefined -fsanitize=address
LDFLAGS += -Wall -Wextra

CFLAGS += -Og -g

DEPFLAGS = -MMD -MP -MF $(@:.o=.d)

OFILES += build/instructions.o

$(PROG): $(OFILES)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^

clean:
	rm -rf build/

build/%.o: %.c
	mkdir -p $(@D)
	$(CC) $(CFLAGS) $(DEPFLAGS) -c -o $@ $<

build/%.o: build/%.c
	mkdir -p $(@D)
	$(CC) $(CFLAGS) $(DEPFLAGS) -I. -c -o $@ $<

build/instructions.c: ../instruction_set.json5
	mkdir -p $(@D)
	./generate_instructions.py --header-name "instructions.h" -o $@ $<

build/instructions.c: generate_instructions.py

-include $(DEPFILES)

