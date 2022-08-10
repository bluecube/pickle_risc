#!/usr/bin/env python3

import tqdm

import math
import heapq
import dataclasses
import itertools
import collections

opcode = object()

r = 3

instructions = {
    "_immediate_alu": {
        "reg": (r, ("reg_to_left_bus", "load_reg")),
        "immediate": (7, ("to_right_bus",)),
        "op": (3, opcode),
    },
    "_3op_alu": {
        "destination": (r, ("load_reg",)),
        "source1": (r, ("reg_to_left_bus",)),
        "source2": (r, ("reg_to_right_bus",)),
        "op": (4, opcode),
    },
    "jl": {
        "offset": (10, ("to_addr_offset",)),
        "link_register": (r, ("load_reg",)),
    },
    "jla": {
        "address": (r, ("reg_to_right_bus",)),
        "link_register": (r, ("load_reg",)),
    },
    "_branch": {
        "offset": (7, ("to_addr_offset",)),
        "link_register": (r, ("load_reg",)),
        "condition": (3, opcode),
    },
    "ldui": {
        "immediate": (9, ("upper_to_left_bus",)),  # Or upper to right bus
        "reg": (r, ("load_reg",)),
    },
    "pop": {
        "destination": (r, ("load_reg",)),
        "address": (r, ("reg_to_right_bus", "load_reg")),
    },
    "push": {
        "source": (r, ("reg_to_left_bus",)),
        "address": (r, ("reg_to_right_bus", "load_reg")),
    },
    "ld": {
        "destination": (r, ("load_reg",)),
        "address": (r, ("reg_to_right_bus",)),
        "offset": (7, ("to_addr_offset",)),  # any other destination type would work too
    },
    "st": {
        "source": (r, ("reg_to_left_bus",)),
        "address": (r, ("reg_to_right_bus",)),
        "offset": (7, ("to_addr_offset",)),  # any other destination type would work too
    },
    "ldcr": {
        "destination": (r, ("load_reg",)),
        "control_register": (3, ("control_register",)),
    },
    "stcr": {
        "source": (r, ("reg_to_left_bus",)),
        "control_register": (3, ("control_register",)),
    },
    "syscall": {
        "code": (7, ("to_right_bus",)),
    },
    "reti": {},
    "break": {}
}

# Cosmetic only: Make pairs of instructions occupy successive encodings
instruction_pairs = [
    ("_immediate_alu", "_3op_alu"),
    ("pop", "push"),
    ("ld", "st"),
    ("ldcr", "stcr"),
]


def capabilities_to_bitsets():
    capabilities_names = set()
    for instr, args in instructions.items():
        for arg_name, (arg_bits, arg_capabilities) in args.items():
            if arg_capabilities is opcode:
                continue
            else:
                if not isinstance(arg_capabilities, tuple):
                    raise ValueError(f"Arg capabilities must be a tuple ({instruction_name}, {arg_name})")

                capabilities_names.update(arg_capabilities)

    global capabilities_enc
    capabilities_enc = {name: 1 << i for i, name in enumerate(capabilities_names)}

    for instr, args in instructions.items():
        for arg_name in args:
            arg_bits, arg_capabilities = args[arg_name]
            if arg_capabilities is opcode:
                continue
            else:
                args[arg_name] = arg_bits, sum(capabilities_enc[cap] for cap in arg_capabilities)


def _pop_from_list(l, predicate):
    """ Find first item in the list that makes predicate return true, modify the list to remove the item,
    return it. """

    for i, v in enumerate(l):
        if predicate(v):
            l.pop(i)
            return v
    return None


def assign_opcodes(instructions, cosmetic_instruction_pairs):
    @dataclasses.dataclass
    class HuffmanItem:
        total_used_bits: int
        opcode_used_bits: int
        min_id: int
        max_id: int
        instructions: dict

        def __lt__(self, other):
            if self.total_used_bits != other.total_used_bits:
                return self.total_used_bits < other.total_used_bits
            elif self.opcode_used_bits != other.opcode_used_bits:
                return self.opcode_used_bits < other.opcode_used_bits
            else:
                return self.max_id < other.min_id

        def __gt__(self, other):
            if self.total_used_bits != other.total_used_bits:
                return self.total_used_bits > other.total_used_bits
            elif self.opcode_used_bits != other.opcode_used_bits:
                return self.opcode_used_bits > other.opcode_used_bits
            else:
                return self.min_id > other.max_id

        @classmethod
        def merge(cls, a, b):
            instructions = {}
            for instr, enc in a.instructions.items():
                instructions[instr] = "0" + enc
            for instr, enc in b.instructions.items():
                instructions[instr] = "1" + enc
            return cls(
                total_used_bits=max(a.total_used_bits, b.total_used_bits) + 1,
                opcode_used_bits=max(a.opcode_used_bits, b.opcode_used_bits) + 1,
                min_id=min(a.min_id, b.min_id),
                max_id=max(a.max_id, b.max_id),
                instructions=instructions
            )

        def __str__(self):
            return str(self.instructions)

    heap = []
    for i, (instr, args) in enumerate(instructions.items()):
        bits_used = 0
        extra_opcode_bits = 0
        for arg, arg_spec in args.items():
            bits_used += arg_spec[0]
            if arg_spec[1] is opcode:
                extra_opcode_bits += arg_spec[0]

        heap.append(HuffmanItem(
            total_used_bits=bits_used,
            opcode_used_bits=extra_opcode_bits,
            min_id=i,
            max_id=i,
            instructions={instr: "x" * extra_opcode_bits}
        ))

    for name1, name2 in cosmetic_instruction_pairs:
        item1 = _pop_from_list(heap, lambda x: name1 in x.instructions)
        item2 = _pop_from_list(heap, lambda x: name2 in x.instructions)
        heap.append(HuffmanItem.merge(item1, item2))

    heapq.heapify(heap)

    ordered_instruction_names = list(instructions.keys())

    while len(heap) > 1:
        left = heapq.heappop(heap)
        right = heapq.heappop(heap)

        # Cosmetics: Push first and last instruction to the beginning/end
        if ordered_instruction_names[0] in right.instructions:
            left, right = right, left
        elif ordered_instruction_names[-1] in left.instructions:
            left, right = right, left

        heapq.heappush(heap, HuffmanItem.merge(left, right))

    huffman_codes = heap[0].instructions

    if True:
        for instr, code in sorted(huffman_codes.items(), key=lambda x: x[1]):
            bits_used = len(code)
            if bits_used > 16:
                raise Exception(f"Instruction {instr} has {bits_used} bits used")
            print(f"{code:7}: {instr} ({bits_used} bits used)")

    return heap[0].instructions


#def _field_bits(mask, n, accumulator=0):
#    if n == 0:
#        yield accumulator
#
#    while mask:
#        # Bit twiddle the lowest bit from the mask:
#        mask_without_lower_bit = mask & (mask - 1)
#        lower_bit = mask ^ mask_without_lower_bit
#
#        # The lower bit can be set:
#        yield from _field_bits(
#            mask_without_lower_bit,
#            n - 1,
#            accumulator | lower_bit
#        )
#
#        # Or the lower bit can be unset: (tail recursion)
#        mask = mask_without_lower_bit


def _field_bits(mask, n):
    nbits = (1 << n) - 1
    inv_mask = ~mask
    for i in range(16 - n):
        shifted = nbits << i
        if shifted & inv_mask:
            continue
        else:
            yield shifted


def _instruction_encodings(
    instruction_name, instruction_args,
    opcode_assignments,
    full_instruction_mask
):
    used_opcode_length = len(opcode_assignments[instruction_name])

    # Opcode is always in top bits

    all_args = []
    total_arg_bits = 0

    for arg_name, (arg_bits, arg_capabilities) in instruction_args.items():
        if arg_capabilities is opcode:
            continue

        total_arg_bits += arg_bits
        all_args.append((arg_name, arg_bits, arg_capabilities))

    wiggle_room_bits = 16 - used_opcode_length - total_arg_bits

    def fit_intervals(remaining_args, starting_at, wiggle_room_available, accumulator):
        if not remaining_args:
            yield accumulator
            return

        arg_name, arg_bits, arg_capabilities = remaining_args[0]
        base_mask = (1 << arg_bits) - 1
        for gap in range(wiggle_room_available + 1):
            yield from fit_intervals(
                remaining_args[1:],
                starting_at + arg_bits + gap,
                wiggle_room_available - gap,
                accumulator + [((instruction_name, arg_name), base_mask << (gap + starting_at), arg_capabilities)]
            )

    for permutation in itertools.permutations(all_args):
        yield from fit_intervals(permutation, 0, wiggle_room_bits, [])


def _all_field_allocations(instructions, opcode_assignments):
    full_instruction_mask = 0xffff

    instruction_encoding_options = [
        list(_instruction_encodings(
            instruction_name, instruction_args,
            opcode_assignments,
            full_instruction_mask
        ))
        for instruction_name, instruction_args in instructions.items()
    ]

    #for x in instruction_encoding_options:
    #    print(x)

    total_count = math.prod(len(x) for x in instruction_encoding_options)

    for specific_encoding in tqdm.tqdm(itertools.product(*instruction_encoding_options), total=total_count):
        yield specific_encoding


def _field_allocation_cost(field_allocation):
    #all_fields = collections.defaultdict(set)
    #for instr_args in field_allocation:
    #    for arg_id, arg_mask, arg_capabilities in instr_args:
    #        all_fields.setdefault(arg_mask, set()).update(arg_capabilities)

    all_fields = collections.defaultdict(int)
    for instr_args in field_allocation:
        for arg_id, arg_mask, arg_capabilities in instr_args:
            all_fields[arg_mask] |= arg_capabilities

    # PyPy doesn't support `int.bit_count(), so we have to implement it manually here
    def bit_count(x):
        for i in itertools.count():
            if x == 0:
                return i
            x = x & (x - 1)
    main_cost = sum(bit_count(x) for x in all_fields.values())

    #main_cost = sum(x.bit_count() for x in all_fields.values())
    return main_cost, len(all_fields)


def _merge_fields(field_allocation):
    merged_fields = {}
    for instr_args in field_allocation:
        for arg_id, arg_mask, arg_capabilities in instr_args:
            decoded_capabilities = {name for name, bit in capabilities_enc.items() if arg_capabilities & bit}
            field_capabilities, field_users = merged_fields.setdefault(arg_mask, (set(), []))
            field_capabilities.update(decoded_capabilities)
            field_users.append(arg_id)

#            try:
#                field_capabilities, field_users = merged_fields[arg_mask]
#                field_capabilities.update(arg_capabilities)
#                field_users.append(arg_id)
#            except KeyError:
#                field_capabilities = set(arg_capabilities)
#                field_users = [arg_id]
#                merged_fields[arg_mask] = (field_capabilities, field_users)

    return merged_fields


def _print_fields(f):
    for mask, (capabilities, users) in sorted(f.items()):
        print(f"{mask:019_b}: " + ", ".join(capabilities))
        print("    " + ", ".join(f"{instr}/{field}" for instr, field in users))


def field_allocations(instructions, opcode_assignments):
    best = None
    best_cost = None
    for fa in _all_field_allocations(instructions, opcode_assignments):
        cost = _field_allocation_cost(fa)
        if best is None or cost < best_cost:
            best = _merge_fields(fa)
            best_cost = cost
        #merged_fields = _merge_fields(fa)
        #cost = (sum(len(x[0]) for x in merged_fields.values()), len(merged_fields))
        #if best is None or cost < best_cost:
        #    best = merged_fields
        #    best_cost = cost
            if True:
                print()
                print("Found new best:", cost)
                _print_fields(best)

    return best

capabilities_to_bitsets()

opcode_assignments = assign_opcodes(instructions, instruction_pairs)
field_allocations(instructions, opcode_assignments)
