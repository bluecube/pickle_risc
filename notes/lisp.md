# Lisp

The OS will most likely have an interpreting Lisp shell.

Potentially later this could be extended to a compiler.

## Design notes

All values are aligned to even addresses

## Pointer tagging

- `0000 0000 0000 0000`: NIL
- `xxxx xxxx xxxx xxx0`: Pointer to object
- `xxxx xxxx xxxx xxx1`: 15bit immediate signed integer

## Objects

Objects are encoded either inline in a pointer, or through a special value of the first
word.

- NIL
    - Encoded inline
- Small integer
    - Encoded inline
- Integer
    - Bignum?
- Float
    - Will this be necessary?
- Char
    - Can we use integers instead?
    - 001 prefix
- Bool
    - Can we use integers instead?
- String
    - Step 2
- Array
    - Step 3
- Symbol
- Closure
    - How do these work exactly?
- Compiled function
    - Used for built-ins
    - Later for compiled code?
- Pair
    - Encoded by not having any of the special values in the first word
