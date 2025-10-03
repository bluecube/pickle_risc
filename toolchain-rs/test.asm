



named_scope: {
        addi r0, 0
        ldi r1, 0x12
        here:
        ldui r2, 0x34
        ori r2, 0x56
        break

        {

        macro test_macro a, b {
                nop
        }

        }
}

{}
        ldui 
        mul_widening_16! r1, r2, 0x123 + named_scope.here - (2 * 0b001)

label:

add r0, r4, r6


macro mul_widening_16 result_hi, result_lo, a, b, b_hi, jump_tmp1, jump_tmp2 {
        and result_lo, r0, r0
        and result_hi, r0, r0
        and b_hi, r0, r0
        ldpc jump_tmp1, loop_top ; Preload loop_top
        ldpc jump_tmp2, skip_add ; Preload skip_add

        loop_top:
        shr a, a ; Shift right a, reading the current bit to C

        bnc jump_tmp2 ; Jump to skip_add if the shifted out bit was zero
        nop ; Nothing to do in the branch delay slot
        add result_lo, result_lo, b ; Add b to result -- lower half
        addc result_hi, b_hi ; Add b to result -- upper half

        skip_add:
        add b, b, b ; Shift the lower half of b left. On the last iteration this is useless, but also harmless
        bnz a, jump_tmp1 ; If r1 is nonzero, jump back to loop_top
        addc b_hi, b_hi ; Delay slot: Shift the upper half of b.
}

macro nop { add r0, r0, r0 }

