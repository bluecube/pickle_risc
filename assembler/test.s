ldui r5, 0xAA
ori r5, 0x55
ldi r2, label
add r0, r2, r5 # Comment
label:
rjmp forward_ref + 1
jmp?z r1
ldcr r7, IntPc
forward_ref:
break
