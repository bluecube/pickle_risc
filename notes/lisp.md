# Lisp

The OS will most likely have an interpreting Lisp shell.

Potentially later this could be extended to a compiler.

Based on [Make a Lisp](https://github.com/kanaka/mal/blob/master/process/guide.md).

## Objects

All objects are aligned to even addresses.
Objects types are encoded either inline in a pointer, or through a special value of the first
word.

The following list is loosely based on MAL's types

- Nil (encoded [inline](#pointer-tagging))
- Small int (encoded [inline](#pointer-tagging))
- Cons
- String
- Symbol
    - pointer into symbol table
- Keyword
    - symbol automatically evaluates to itself
- Atom (MAL step 6)
- Hash
- List (?)
- Native function
- Closure (MAL step 5)
- Bool?
    - MAL uses separate true and false types
    - common lisp seems to have symbol t that evaluates to itself
    - maybe use true and false symbols evaluating to themselves, treat false and nil as false, everything else as true

## Pointer tagging

- `0000 0000 0000 0000`: NIL
- `xxxx xxxx xxxx xxx0`: Pointer to object
- `xxxx xxxx xxxx xxx1`: 15bit immediate signed integer

## Allocation / GC
- Copying GC
- Two generations
    - Object mutability is a problem:
        - All values could be RO
            - Too limiting?
        - Mutation of an older generation object could cause an immediate GC of the nursery area (with one extra root)
        - We could keep a list of old generation objects that will serve as additional roots for nursery GC
            - This could be just a Lisp list held with a single global pointer
- Eventually the "new" space could be limited to only two pages with some and manipulation of memory maps
    - Allows to use almost the full 64k space for the working data, not just half
        - Additional 64k of physical memory would be necessary for GC
            - Only paid once for all interpreter instances, if the OS supports it (no process switch during GC)
- Eventually the OS could remap old generations as RO and provide a callback to the GC in case there is a change to older data.
