; Copy Count words from Src to Dst
; Clobbers JumpTmp, WordTmp
; At the end Count == 0, Src and Dst point one element past the buffers
memcpy_w:
ldp JumpTmp, end
bz Count, JumpTmp ; Early exit if count is zero
ldp JumpTmp, loop
addi Dst, -1 ; Shift Dst to alow using the pre-increment store
loop:
ld WordTmp, Src + 0 ; Load next word to move
st+ Dst, WordTmp ; Move Dst pointer and store word
addi Count, -1
bnz Count, JumpTmp
addi Src, 1 ; Delay slot: Move Src pointer
end:



; Strings use extended pointers, where the low bit of the pointer determines
; address within a word.



; Load a byte from an extended pointer Ptr to a low byte of Dst.
; Clobbers Ptr, Tmp, C
; 6 cycles
load_b:
shr Ptr, Ptr ; Convert from extended to normal pointer, store the byte index in C
ld Dst, Ptr + 0
shr8 Tmp, Dst ; Tmp now contains the top byte
pack Dst, Dst, r0 ; Dst now contains the low byte
cadd Dst, Tmp, r0 ; Conditionally move Dst into Tmp


; Store a byte from low byte of Src to an extended pointer address Ptr.
; Clobbers Ptr, Tmp, Tmp_high, C
; 9 cycles
store_b:
shr Ptr, Ptr ; Convert from extended to normal pointer, store the byte index in C
ld Tmp, Ptr + 0 ; Load the existing value
pack Tmp_high, Tmp, Src ; Tmp_high is the loaded word with upper byte replaced by Src
shr8 Tmp, Tmp ; Move the loaded high word down, preparing for pack
pack Tmp, Src, Tmp ; Tmp is the loaded word with lower byte replaced by Src
cadd Tmp, Tmp_high, r0 ; Replace the value in Tmp with Tmp_high, if the C bit was set
st Ptr + 0, Tmp


; Copy Count bytes from Src to Dst
; Clobbers Count, Src, Dst, JumpTmp, WordTmp
memcpy_b:
shr SrcW, Src ; Convert from extended to normal pointer, store the byte index in C
ldpc Tmp, src_aligned
brnc Tmp ; Skip the src alignment if it is alread at word boundary
nop ; Delay slot
load_b! Tmp, Src, Tmp2
store_b! Dst, Tmp, Tmp2, Tmp3
addi Src, 1
addi Dst, 1
addi Count, -1
shr SrcW, Src

src_aligned: ; Now Src is aligned to word boundary (= even)
shr DstW, Dst

ldpc Tmp, unaligned
brc Tmp
nop

shr CountW, Count ; CountW = Count/2 for the word memcpy
andi Count, 1 ; Count % 2, the remaining byte after word memcpy

memcpy_w! DstW, SrcW, CountW, Tmp, Tmp2

ldpc Tmp, end
bz Count, Tmp

add Src, SrcW, SrcW
add Dst, DstW, DstW

ldpc Tmp, end
jal r0, Tmp
nop

memcpy_b_unaligned! Src, Dst ; TODO

end:
