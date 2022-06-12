ldui r5, included_data
ori r5, 0x55
.include "test_include.s"
ldi r2, label
add r0, r2, r5 # Comment
label:
rjmp forward_ref + 1
jmp?z r1
ldcr r7, IntPc
forward_ref:
break

table:
.db 1, 2, 3
.db "Hello world!\n\0"
.dw 0xfedc, 0x1234, 1 + 1
.dd -2, 0x1eadbeef
