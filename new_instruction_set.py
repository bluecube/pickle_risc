#!/usr/bin/env python3

import tqdm

import math
import copy
import heapq
import dataclasses
import itertools
import collections
import os
import sys
import multiprocessing
import functools
import contextlib

opcode = object()

r = 3

caps = {name: 1 << i for i, name in enumerate([
    "reg_to_left_bus", "reg_to_right_bus", "load_reg",
    "to_right_bus", "upper_to_left_bus",
    "to_addr_offset",
    "control_register"
])}

instructions = {
    "_immediate_alu": {
        "reg": (r, caps["reg_to_left_bus"] | caps["load_reg"]),
        "immediate": (7, caps["to_right_bus"]),
        "op": (3, opcode),
    },
    "_3op_alu": {
        "destination": (r, caps["load_reg"]),
        "source1": (r, caps["reg_to_left_bus"]),
        "source2": (r, caps["reg_to_right_bus"]),
        "op": (4, opcode),
    },
    "jl": {
        "offset": (10, caps["to_addr_offset"]),
        "link_register": (r, caps["load_reg"]),
    },
    "jla": {
        "address": (r, caps["reg_to_right_bus"]),
        "link_register": (r, caps["load_reg"]),
    },
    "_branch": {
        "offset": (7, caps["to_addr_offset"]),
        "condition": (3, opcode),
    },
    "ldui": {
        "immediate": (9, caps["upper_to_left_bus"]),  # Or upper to right bus
        "reg": (r, caps["load_reg"]),
    },
    "_pop_push": {
        "store_flag": (1, opcode),
        "data": (r, caps["load_reg"] | caps["reg_to_left_bus"]),
        "address": (r, caps["reg_to_right_bus"] | caps["load_reg"]),
    },
    "_ld_st": {
        "store_flag": (1, opcode),
        "data": (r, caps["load_reg"] | caps["reg_to_left_bus"]),
        "address": (r, caps["reg_to_right_bus"]),
        "offset": (7, caps["to_addr_offset"]),  # any other destination type would work too
    },
    "_ldcr_stcr": {
        "store_flag": (1, opcode),
        "data": (r, caps["load_reg"] | caps["reg_to_left_bus"]),
        "control_register": (3, caps["control_register"]),
    },
    "syscall": {
        "code": (7, caps["to_right_bus"]),
    },
    "reti": {},
    "break": {}
}

full_mask = 0xffff


def assign_opcodes(instructions, cosmetic_instruction_pairs, print_fun=None):
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

    if print_fun:
        for instr, code in sorted(huffman_codes.items(), key=lambda x: x[1]):
            bits_used = len(code)
            if bits_used > 16:
                raise Exception(f"Instruction {instr} has {bits_used} bits used")
            print_fun(f"{code:7}: {instr} ({bits_used} bits used)")

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


def _parallel_product_min_map_fun(remaining_iterables, key_fun, task_tuple):
    key_part1 = key_fun[0](task_tuple)

    it = itertools.product(*remaining_iterables)
    value = next(it)
    best = task_tuple + value
    best_key = key_fun[1](value, copy.copy(key_part1))
    for remaining_tuple in it:
        value = task_tuple + remaining_tuple
        key = key_fun[1](remaining_tuple, copy.copy(key_part1))
        if key < best_key:
            best = value
            best_key = key

    return (best, best_key)


def _parallel_product_min(iterables, key_fun, improvement_callback, chunk_size=1024, granularity=1024 * 2):
    iterables = list(iterables)
    total_count = math.prod(len(x) for x in iterables)

    target_task_count = os.cpu_count() * chunk_size * granularity
    split_index = len(iterables)
    task_count = 1

    for i in range(len(iterables)):
        if task_count >= target_task_count:
            split_index = i
            break
        else:
            task_count *= len(iterables[i])

    task_iterables = iterables[0:split_index]
    remaining_iterables = iterables[split_index:]

    with multiprocessing.Pool() as pool:
        best = None
        best_key = None
        for value, key in tqdm.tqdm(
            pool.imap_unordered(
                functools.partial(_parallel_product_min_map_fun, remaining_iterables, key_fun),
                itertools.product(*task_iterables),
                chunksize=chunk_size
            ),
            total=task_count,
            unit_scale=total_count//task_count,
            smoothing=0.00001,
            dynamic_ncols=True,
        ):
            if best is None or key < best_key:
                best = value
                best_key = key
                improvement_callback(best, best_key)

    return best, best_key


# PyPy doesn't support `int.bit_count(), so we have to implement it manually here
def _bit_count(x):
    for i in itertools.count():
        if x == 0:
            return i
        x = x & (x - 1)


def _field_allocation_cost_part1(field_allocation_part1):
    all_fields = collections.defaultdict(int)
    for instr_args in field_allocation_part1:
        for arg_id, arg_mask, arg_capabilities in instr_args:
            all_fields[arg_mask] |= arg_capabilities

    return all_fields


def _field_allocation_cost_part2(field_allocation_part2, all_fields):
    for instr_args in field_allocation_part2:
        for arg_id, arg_mask, arg_capabilities in instr_args:
            all_fields[arg_mask] |= arg_capabilities

    main_cost = sum(_bit_count(x) for x in all_fields.values())
    return main_cost, len(all_fields)


def _merge_fields(field_allocation):
    merged_fields = {}
    for instr_args in field_allocation:
        for arg_id, arg_mask, arg_capabilities in instr_args:
            decoded_capabilities = {name for name, bit in caps.items() if arg_capabilities & bit}
            field_capabilities, field_users = merged_fields.setdefault(arg_mask, (set(), []))
            field_capabilities.update(decoded_capabilities)
            field_users.append(arg_id)

    return merged_fields


def field_allocations(instructions, opcode_assignments, print_fun=None):
    full_instruction_mask = 0xffff

    instruction_encoding_options = [
        list(_instruction_encodings(
            instruction_name, instruction_args,
            opcode_assignments,
            full_instruction_mask
        ))
        for instruction_name, instruction_args in instructions.items()
    ]

    if print_fun:
        def improvement_callback(field_allocation, cost):
            print_fun()
            print_fun()
            print_fun("Found new best:", cost)
            merged = _merge_fields(field_allocation)
            for mask, (capabilities, users) in sorted(merged.items()):
                print_fun(f"{mask:019_b}: " + ", ".join(capabilities))
                print_fun("    " + ", ".join(f"{instr}/{field}" for instr, field in users))
    else:
        def improvement_callback(*args, **kwargs):
            pass

    return _parallel_product_min(
        instruction_encoding_options,
        key_fun=(_field_allocation_cost_part1, _field_allocation_cost_part2),
        improvement_callback=improvement_callback,
    )


@contextlib.contextmanager
def printing():
    if len(sys.argv) == 2:
        print(f"Opening {sys.argv[1]} for output duplication")
        with open(sys.argv[1], "w", buffering=1) as fp:
            def p(*args, **kwargs):
                print(*args, **kwargs)
                print(*args, **kwargs, file=fp)
            yield p
    else:
        yield print


with printing() as print_fun:
    opcode_assignments = assign_opcodes(instructions, instruction_pairs, print_fun)
    field_allocations(instructions, opcode_assignments, print_fun)
