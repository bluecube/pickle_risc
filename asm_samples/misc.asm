; Calculate Dst = sext(Src)
; 5 cycles

pack Dst, r0, Src ; Dst = Src << 8
add r0, Dst, Dst ; Shift Dst to the right to load the high bit to C
cadd Dst, -1 ; If the high bit in C was 1, then add 0xffff to Dst, ensuring its lower byte is 0xff
; Now  lower byte of Dst is filled with the sign extending bit value
pack Dst, Src, Dst ; Pack the original low  byte from Src with the sign extending byte
