; Calculate r3 = r1 * r2, unsigned, discarding upper 16bit of the multiplication
; Clobbers r1, r2, r4
; For better performance, r1 < r2 should hold

mul_16:
and r3, r0, r0 ; Clear r3
ldpc r4, loop_top ; Preload loop_top to r4

loop_top:
shr r1, r1 ; Shift right r1, reading the current bit to C
cadd r3, r3, r2 ; Add r2 to result if the shifted out bit was set
bnz r1, r4 ; If r1 is nonzero, jump back to next loop iteration
add r2, r2, r2 ; Delay slot: Shift r2 left. On the last iteration this does nothing, because r2 is already zero





; Calculate r5,r4 = r1 * r2, unsigned, with r5 containing the upper 16bit and r4 containing the lower 16 bit
; Clobbers r1, r2, r3, r6, r7
; For better performance, r1 < r2 should hold

; r3,r2: 32bit pair of extended r2, being shifted up

macro mul_widening_16 {
and r3, r0, r0 ; Clear r3
and r4, r0, r0 ; Clear r4
and r5, r0, r0 ; Clear r5
and r6, r0, r0 ; Clear r6
ldpc r6, loop_top ; Preload loop_top to r6
ldpc r7, skip_add ; Preload skip_add to r7

loop_top:
shr r1, r1 ; Shift right r1, reading the current bit to C

bnc r7 ; Jump to skip_add if the shifted out bit was zero
nop ; Nothing to do in the branch delay slot
add r4, r4, r2 ; Add r3,r2 to result -- lower half
addc r5, r3 ; Add r3,r2 to result -- upper half

skip_add:
add r2, r2, r2 ; Shift the lower half of r3,r2 left. On the last iteration this is useless, but also harmless
bnz r1, r6 ; If r1 is nonzero, jump back to loop_top
addc r3, r3 ; Delay slot: Shift the upper half of r3,r2.

}
