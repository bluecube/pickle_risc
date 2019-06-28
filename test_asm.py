import trilo8bit.assembler as asm
from trilo8bit.assembler.instructions import *

@asm.function
def fizzbuzz():
    """ Takes no arguments, returns no value, prints fizzbuzz using the puts and putc functions. """
    _push_a() # Save the return address
    push(0) # Push the loop counter

    asm.label("loop")

    inc() # Increment the loop counter

    push(0) # Flag determining whether or not the number should be printed

    over() # Duplicate the loop counter on top of the stack for modulo call
    push(3) # Fizz is printed when number is divisible by 3
    modulo8.call()
    branch("not_fizz")
    _write_data(fizz)
    load_immediate(1) # Overwrite the flag with true value
    asm.label("not_fizz")

    over() # Duplicate the loop counter on top of the stack for modulo call
    push(5)# Buzz is printed when number is divisible by 5
    modulo8.call()
    branch("not_buzz")
    _write_data(buzz)
    load_immediate(1) # Overwrite the flag with true value
    asm.label("not_buzz")

    branch("not_number")
    dup() # Duplicate the counter as an argument for the function call
    write_num8.call() # Write counter to the output
    asm.label("not_number")

    _push(ord("\n"))
    putc.call()

    dup() # duplicate loop counter for end of loop test
    _push(100) # Push the loop iteration count
    sub() # Subtract the loop counter
    bit_not() # Negate the outcome
    branch("end") # Jump to the end if the value was equal

    load_immediate_a_low(lo8("loop")) # Load the loop start address (it is further than -8 bytes)
    load_immediate_a_high(hi8("loop"))
    swap_pc() # Since we ignore the return value at the called location, it's effectively only a jump

    asm.label("end")
    drop() # Drop the loop counter from the stack
    _ret()

    # Define the string constants, they will live in instruction space (exactly where defined here).
    # This means that they have to be in an unreachable part of the function, otherwise we would try to execute them.
    fizz = asm.data("Fizz")
    buzz = asm.data("Buzz")

@asm.macro # Assembler macros are the same as functions (they create label scope), but can only be inlined (using the __call__ method)
def _memcpy8():
    """ Get source address in B, target address in A and number of elements to copy on the stack top.
    Moves A and B one element past the source and target areas, keeps count of 255 on the stack.
    Zero count is valid and causes no bytes to be copied. """

    dup()
    jump("condition")
    asm.label("loop")
    load_deep_b_inc()
    store_deep_a_inc()
    asm.label("condition")
    branch_dec("loop")
    nip()

    # 4 + 3*n ticks

@asm.macro
def _strlen8():
    """ Get source address in B, pushes length of B modulo 256 to a stack.
    Moves B to the terminating zero.
    Clobbers D register. """
    push(0)
    dup()
    jump("condition")
    asm.label("loop")
    inc()
    asm.label("condition")
    load_deep_b_inc()
    branch_deep("loop")
    nip()

@asm.macro
def _push(value):
    """ Push an 8bit value onto the stack using push and load_immediate pair """
    push(0)
    load_immediate(value)

@asm.macro
def _push_a():
    """ Save the value of A on the stack. (Function prologue) """
    push_a_high()
    push_a_low()

@asm.macro
def _ret():
    """ Load return value from stack and return (Function epilogue) """
    set_a_low()
    set_a_high()
    swap_pc()

def _write_data(data_label): # Not a macro, because we want to reach the enclosing scope's label
    """ Write out data bound to a label. Clobbers A and B register. """
    _set_b_from_label(data_label)
    _push(len(data_label.data))
    puts.call()

def _set_b_from_label(data_label):
    """ Load address of a label into B """
    load_immediate_b_high(hi8(data_label))
    load_immediate_b_low(lo8(data_label))

@asm.macro
def _halt():
    """ Keep sleeping the cpu in an endless loop """
    asm.label("loop")
    sleep()
    jump("loop")

def _load_pointer(label):
    """ Loads pointer into A, its address into B. """
    _set_b_from_label(label)
    push(0) # Make space for the high byte on the stack
    load_b_inc() # Load the high byte of the pointer
    set_a_high() # set the high byte in A
    push(0) # Make space for the low byte
    load_b_dec() # Load the low byte and restore B back to the high byte of the pointer
    set_a_low() # set the low byte in A

@asm.function
def write_num8():
    """ Write an 8bit number to console as decimal """

@asm.function
def puts():
    """ Gets pointer to memory in B, number of bytes on stack. """
    _push_a() # Save return address

    push_b_high() # Save the data pointer
    push_b_low()
    _load_pointer("console_pointer") # Load console pointer to A
    set_b_low() # restore the data pointer in B
    set_b_high()

    push(0)
    load_stack_relative(1) # Load the size to be copied from the depths
        #TODO: This is ugly, we burry the arguments under return value and then have to dig it up again

    _memcpy8() # Copy the actual data
    drop() # Drop the remainders of the count from memcpy

    # A contains the new position of the console pointer
    # We reload the address of stack pointer from the label and store the modified A

    _set_b_from_label("console_pointer")
    push_a_high() # Store the modified high byte of the pointer back
    store_b_inc()
    drop() # Drop the value
    push_a_low() #... low byte
    store_b_dec()
    drop() # Drop the value

    # Load the return address back from stack into A
    set_a_low()
    set_a_high()

    drop() # Drop the argument

    swap_pc() # Return

@asm.function
def putc():
    _push_a() # Save return address
    _load_pointer("console_pointer") # Load console pointer to A, its address into B

    push(0)
    load_stack_relative(1) # Load the byte to be written from the stack
        #TODO: This is ugly, we burry the arguments under return value and then have to dig it up again

    store_a_inc() # Store the character in output buffer
    drop() # Drop the duplicate

    push_a_high() # Store the modified high byte of the pointer back
    store_b_inc()
    drop() # Drop the value
    push_a_low() #... low byte
    store_b_dec()
    drop() # Drop the value

    # Load the return address back from stack into A
    set_a_low()
    set_a_high()

    drop() # Drop the argument

    swap_pc() # Return

@asm.function
def divmod8():
    """ a b -> q, r  -- a / b = b * q + r """

    set_b_low() # Using b low as a storage for divisor
    load_immediate_b_high(0) # Using b high as a storage for quotient

    push(0)
    swap()

    asm.label("loop")

    bit_shift_left() # Shift one bit from the dividend, storing the shifted bit in C
    swap() # swap dividend and remainder
    bit_shift_left_carry() # Shift remainder left, adding the bit carried from dividend

    branch_if_carry("subtract") # If the shift left overflowed, we know that remainder is larger than divisor

    dup() # Duplicate new remainder
    push_b_low() # Push the divisor
    compare_lt()
    branch("no_subtract") #if remainder is lower than divisor, skip the subtraction back and do a next bit

    asm.label("subtract")
    push_b_low() # Push the divisor again
    sub() # Subtract divisor from the remainder. This behaves correctly even when the remainder calculation overflowed


    asm.label("no_subtract")

    jump("loop") # Go back

@asm.function
def main():
    fizzbuzz.call()
    _halt()

    console_buffer = asm.byte_data(b"\0" * 1024)
    console_pointer = asm.data(buffer.location, 16, name="console_pointer") # Needs an explicit name to be accessible from other functions

print(main.assemble())
