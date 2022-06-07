ldui r5, 0xaa
ori r5, 0x55
ldi r2, 10
add r0, r2, r5 # Comment
label:
rjmp 0x12
jmp?z r1
ldcr r7, IntPc
break
