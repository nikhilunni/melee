"""Read-only HSD simulation snapshots (NTSC-U 1.02, 32-bit big-endian ABI).

Layouts: sysdolphin/baselib/psstructs.h, HSD_Generator (0x94),
HSD_Particle (0x98), HSD_psAppSRT (0xA4), and list.h (0x08).
Particle links are the sixteen global lists in particle.c, not child lists
inside a generator. Scalar floats are read as words to retain NaN payloads.
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol

from jobjdump import f32_value
import symbols


class Memory(Protocol):
    def read_u8(self, addr: int) -> int: ...
    def read_u32(self, addr: int) -> int: ...


MEM1_LO, MEM1_HI = 0x80000000, 0x81800000
GENERATOR_SIZE, PARTICLE_SIZE, APPSRT_SIZE = 0x94, 0x98, 0xA4
MAX_OBJECTS, MAX_PROGRAMS = 65536, 65536


@dataclass(frozen=True)
class Field:
    name: str
    offset: int
    kind: str = "u32"
    count: int = 1


# psstructs.h: HSD_Generator, all initialized simulation members through +0x5C.
GENERATOR_FIELDS = (
    Field("next", 0x00), Field("kind", 0x04), Field("random", 0x08, "f32"),
    Field("count", 0x0C, "f32"), Field("jobj", 0x10),
    Field("generator_life", 0x14, "u16"), Field("type", 0x16, "u16"),
    Field("bank", 0x18, "u8"), Field("link", 0x19, "u8"),
    Field("texture_group", 0x1A, "u8"), Field("id", 0x1C, "u16"),
    Field("particle_life", 0x1E, "u16"), Field("program", 0x20),
    Field("position", 0x24, "f32", 3), Field("velocity", 0x30, "f32", 3),
    Field("gravity", 0x3C, "f32"), Field("friction", 0x40, "f32"),
    Field("size", 0x44, "f32"), Field("radius", 0x48, "f32"),
    Field("angle", 0x4C, "f32"), Field("child_count", 0x50),
    Field("appsrt", 0x54), Field("user_functions", 0x58), Field("callback", 0x5C),
)
# psstructs.h: HSD_Particle. GXColor is four consecutive bytes.
PARTICLE_FIELDS = (
    Field("next", 0x00), Field("kind", 0x04), Field("bank", 0x08, "u8"),
    Field("texture_group", 0x09, "u8"), Field("pose", 0x0A, "u8"),
    Field("palette", 0x0B, "u8"), Field("size_count", 0x0C, "u16"),
    Field("primary_count", 0x0E, "u16"), Field("environment_count", 0x10, "u16"),
    Field("primary_color", 0x12, "u8", 4), Field("environment_color", 0x16, "u8", 4),
    Field("command_wait", 0x1A, "u16"), Field("loop_count", 0x1C, "u8"),
    Field("link", 0x1D, "u8"), Field("id", 0x1E, "u16"), Field("program", 0x20),
    Field("pc", 0x24, "u16"), Field("mark_pc", 0x26, "u16"),
    Field("loop_pc", 0x28, "u16"), Field("life", 0x2A, "u16"),
    Field("velocity", 0x2C, "f32", 3), Field("gravity", 0x38, "f32"),
    Field("friction", 0x3C, "f32"), Field("position", 0x40, "f32", 3),
    Field("size", 0x4C, "f32"), Field("rotation", 0x50, "f32"),
    Field("alpha_compare_count", 0x54, "u16"), Field("alpha_compare_mode", 0x56, "u8"),
    Field("alpha_compare_parameters", 0x57, "u8", 2), Field("point_joint_offset", 0x59, "u8"),
    Field("material_count", 0x5A, "u16"), Field("ambient_count", 0x5C, "u16"),
    Field("rotation_count", 0x5E, "u16"), Field("size_target", 0x60, "f32"),
    Field("rotation_target", 0x64, "f32"), Field("primary_remaining", 0x68, "u16"),
    Field("environment_remaining", 0x6A, "u16"), Field("primary_target", 0x6C, "u8", 4),
    Field("environment_target", 0x70, "u8", 4), Field("material_remaining", 0x74, "u16"),
    Field("ambient_remaining", 0x76, "u16"), Field("alpha_compare_remaining", 0x78, "u16"),
    Field("alpha_compare_targets", 0x7A, "u8", 2), Field("material", 0x7C, "u8", 2),
    Field("ambient", 0x7E, "u8", 2), Field("material_target", 0x80, "u8", 2),
    Field("ambient_target", 0x82, "u8", 2), Field("trail", 0x84, "f32"),
    Field("generator", 0x88), Field("appsrt", 0x8C), Field("userdata", 0x90),
    Field("callback", 0x94),
)
APPSRT_FIELDS = (
    Field("next", 0), Field("generator", 4), Field("translation", 8, "f32", 3),
    Field("rotation", 0x14, "f32", 4), Field("scale", 0x24, "f32", 3),
    Field("status", 0x30, "u8"), Field("frame_count", 0x31, "u8"),
    Field("use_count", 0x32, "u16"), Field("matrix", 0x34, "f32", 12),
    Field("scale_x", 0x64, "f32"), Field("scale_y", 0x68, "f32"),
    Field("unknown_float", 0x6C, "f32", 12), Field("free_callback", 0x9C),
    Field("id", 0xA0, "u16"), Field("unknown_byte", 0xA2, "u8"),
)


def check_pointer(pointer: int, size: int, alignment: int = 4) -> None:
    if pointer % alignment or not MEM1_LO <= pointer <= MEM1_HI - size:
        raise ValueError(f"invalid MEM1 pointer 0x{pointer:08X} (size 0x{size:X})")


def read_fields(mem: Memory, pointer: int, fields: tuple[Field, ...]) -> dict:
    values = {}
    for field in fields:
        stride = {"u8": 1, "u16": 2, "u32": 4, "f32": 4}[field.kind]
        components = []
        for index in range(field.count):
            address = pointer + field.offset + index * stride
            if stride == 4:
                value = mem.read_u32(address)
            elif stride == 2:
                value = mem.read_u8(address) << 8 | mem.read_u8(address + 1)
            else:
                value = mem.read_u8(address)
            components.append(value)
        values[field.name] = components if field.count > 1 else components[0]
    return values


def walk_list(mem: Memory, head: int, size: int, seen: set[int] | None = None) -> list[int]:
    """Head then next (+0); reject cycles/shared nodes instead of truncating."""
    seen = set() if seen is None else seen
    pointers = []
    while head:
        check_pointer(head, size)
        if head in seen or len(seen) >= MAX_OBJECTS:
            raise ValueError(f"cyclic, shared, or oversized list at 0x{head:08X}")
        seen.add(head)
        pointers.append(head)
        head = mem.read_u32(head)
    return pointers


def generator_aux_fields(generator_type: int) -> tuple[Field, ...]:
    """Active union member; generator.c:hsd_8039F05C switch(type & 15)."""
    shape = generator_type & 15
    names = {
        0: ("minimum_angle", "maximum_angle"),
        1: ("line_x", "line_y", "line_z"),
        2: ("tornado_velocity",),
        3: ("minimum_angle", "maximum_angle"),
        4: ("minimum_angle", "maximum_angle"),
        5: ("x", "y", "z", "xx", "xy", "xz", "yx", "yy", "yz", "zx", "zy", "zz"),
        6: ("minimum_angle", "maximum_angle", "height"),
        7: ("minimum_angle", "maximum_angle", "height"),
        8: ("speed", "latitude_mid", "latitude_range", "longitude_mid", "longitude_range"),
    }.get(shape, ())
    fields = tuple(Field(f"aux.{name}", 0x60 + 4 * i, "f32") for i, name in enumerate(names))
    return fields + ((Field("aux.flags", 0x90, "u16"),) if shape == 5 else ())


def program_range(mem: Memory, bank: int) -> tuple[int, range]:
    """Recover the real table bounds from psInitDataBankLoad (0x803984F4).

    For v40..43 ptclref is deliberately biased before the allocation:
    table = header + 12 - first_id * 4, upper = first_id + count.
    Search header candidates backward from the real table end, validating
    both header words before dereferencing any descriptor-table entry.
    """
    if not 0 <= bank < 65:
        raise ValueError(f"invalid particle bank {bank}")
    upper = mem.read_u32(symbols.addr("psCmdListArray") + 4 * bank)
    table = mem.read_u32(symbols.addr("ptclref_804D0E5C") + 4 * bank)
    if upper > MAX_PROGRAMS:
        raise ValueError(f"oversized particle bank {bank}: {upper}")
    if not upper:
        return table, range(0)
    # Version zero has no ID bias and an eight-byte header.
    if MEM1_LO <= table - 8 <= MEM1_HI - 8 and table % 4 == 0:
        if mem.read_u32(table - 8) >> 16 == 0 and mem.read_u32(table - 4) == upper:
            check_pointer(table, upper * 4)
            return table, range(upper)
    table_end = table + upper * 4
    check_pointer(table_end - 4, 4)
    for count in range(1, upper + 1):
        header = table_end - 12 - count * 4
        if header < MEM1_LO:
            break
        if mem.read_u32(header) >> 16 not in (0x40, 0x41, 0x42, 0x43):
            continue
        first_id = upper - count
        if mem.read_u32(header + 4) == first_id and mem.read_u32(header + 8) == count:
            check_pointer(header + 12, count * 4)
            return table, range(first_id, upper)
    raise ValueError(f"cannot recover particle bank {bank} header (table 0x{table:08X}, upper {upper})")


def program_indices(mem: Memory, bank: int) -> dict[int, int]:
    """Match relocated cmdList (+0x3C) only within the bank's actual ID range."""
    table, indices = program_range(mem, bank)
    programs = {}
    for index in indices:
        descriptor = mem.read_u32(table + index * 4)
        if descriptor:
            check_pointer(descriptor, 0x3D)
            programs.setdefault(descriptor + 0x3C, index)
    return programs


def _object(mem: Memory, pointer: int, size: int, fields: tuple[Field, ...]) -> dict:
    check_pointer(pointer, size)
    return {"pointer": pointer, "fields": read_fields(mem, pointer, fields),
            "raw_hex": bytes(mem.read_u8(pointer + i) for i in range(size)).hex()}


def snapshot(mem: Memory) -> dict:
    """Capture generator order, all 16 particle orders, and referenced state.

    gen at particle +0x88 is optional (ordinary particles need not retain it).
    Program PCs are byte offsets, not absolute addresses. Raw bytes and pointers
    are diagnostic only; portable records use bank IDs and list ordinals.
    """
    generators = []
    head = mem.read_u32(symbols.addr("hsd_804D78FC"))
    for pointer in walk_list(mem, head, GENERATOR_SIZE):
        generator_type = read_fields(mem, pointer, (Field("type", 0x16, "u16"),))["type"]
        fields = GENERATOR_FIELDS + generator_aux_fields(generator_type)
        generators.append(_object(mem, pointer, GENERATOR_SIZE, fields))
    lists, seen = [], set()
    for link in range(16):
        head = mem.read_u32(symbols.addr("hsd_804D0908") + 4 * link)
        particles = [_object(mem, p, PARTICLE_SIZE, PARTICLE_FIELDS)
                     for p in walk_list(mem, head, PARTICLE_SIZE, seen)]
        for particle in particles:
            if particle["fields"]["link"] != link:
                raise ValueError(f"particle list {link} contains link {particle['fields']['link']}")
        lists.append(particles)
    objects = generators + [p for group in lists for p in group]
    banks = {obj["fields"]["bank"] for obj in objects}
    programs = {bank: program_indices(mem, bank) for bank in banks}
    for obj in objects:
        fields = obj["fields"]
        program = fields["program"]
        if program:
            check_pointer(program, 1, alignment=1)
        obj["program_kind"] = programs[fields["bank"]].get(program)
    app_pointers = dict.fromkeys(obj["fields"]["appsrt"] for obj in objects if obj["fields"]["appsrt"])
    appsrts = [_object(mem, p, APPSRT_SIZE, APPSRT_FIELDS) for p in app_pointers]
    pending = []
    for pointer in walk_list(mem, mem.read_u32(symbols.addr("hsd_804D78F4")), 8):
        pending.append({"pointer": pointer, "generator": mem.read_u32(pointer + 4)})
    joint_pointers = list(dict.fromkeys(
        [obj["fields"]["jobj"] for obj in generators if obj["fields"]["jobj"]]
        + [mem.read_u32(symbols.addr("hsd_804D08E8") + 4 * i) for i in range(8)]
    ))
    # Cached JObj data only: do not invoke matrix setup or traverse other nodes.
    joint_fields = (Field("next", 8), Field("parent", 0x0C), Field("child", 0x10),
                    Field("flags", 0x14), Field("rotation", 0x1C, "f32", 4),
                    Field("scale", 0x2C, "f32", 3), Field("translation", 0x38, "f32", 3),
                    Field("matrix", 0x44, "f32", 12), Field("aobj", 0x7C))
    joints = [_object(mem, p, 0x88, joint_fields) for p in joint_pointers if p]
    globals_ = {}
    for name in ("hsd_804D78D8", "hsd_804D78DA", "numPeakParticles", "hsd_804D78DE",
                 "hsd_804D78E0", "hsd_804D78E2", "hsd_804D78E4", "lbl_804D6368"):
        globals_[name] = read_fields(mem, symbols.addr(name), (Field(name, 0, "u16"),))[name]
    for name in ("psCallback", "hsd_804D78E8", "hsd_804D78EC", "hsd_804D7900", "hsd_804D78F8"):
        globals_[name] = mem.read_u32(symbols.addr(name))
    return {"seed": mem.read_u32(symbols.addr("seed")), "generators": generators,
            "particle_lists": lists, "appsrts": appsrts, "pending_generators": pending,
            "globals": globals_, "joints": joints}


_POINTER_FIELDS = {"next", "jobj", "program", "appsrt", "user_functions", "callback",
                   "generator", "userdata", "free_callback"}


def _scalar_fields(prefix: str, obj: dict, fields: tuple[Field, ...]) -> dict:
    state = {}
    for field in fields:
        if field.name in _POINTER_FIELDS:
            continue
        values = obj["fields"][field.name]
        for index, value in enumerate(values if field.count > 1 else [values]):
            path = f"{prefix}.{field.name}" + (f"[{index}]" if field.count > 1 else "")
            state[path] = f32_value(value) if field.kind == "f32" else {"t": "u", "v": value}
    return state


def record(frame: int, captured: dict) -> dict:
    """One canonical melee-diff record, phase particles, per completed tick."""
    if not 0 <= frame < 1 << 64:
        raise ValueError("frame must fit a u64")
    generators = captured["generators"]
    generator_ids = {obj["pointer"]: i for i, obj in enumerate(generators)}
    app_ids = {obj["pointer"]: i for i, obj in enumerate(captured["appsrts"])}
    state = {"rng.seed": {"t": "u", "v": captured["seed"]},
             "particles.generator_count": {"t": "u", "v": len(generators)},
             "particles.family_id_counter": {"t": "u", "v": captured["globals"]["lbl_804D6368"]}}
    def reference(path, pointer, indices):
        state[path] = {"t": "u", "v": indices[pointer]} if pointer in indices else {"t": "null"}
    def add_object(prefix, obj, fields):
        state.update(_scalar_fields(prefix, obj, fields))
        kind = obj.get("program_kind")
        if "program" in obj["fields"]:
            state[f"{prefix}.program_kind"] = {"t": "u", "v": kind} if kind is not None else {"t": "null"}
            state[f"{prefix}.program_resolved"] = {"t": "u", "v": int(kind is not None)}
        if "generator" in obj["fields"]:
            reference(f"{prefix}.generator_index", obj["fields"]["generator"], generator_ids)
        if "appsrt" in obj["fields"]:
            reference(f"{prefix}.appsrt_index", obj["fields"]["appsrt"], app_ids)
    for index, obj in enumerate(generators):
        add_object(f"particles.generator[{index}]", obj,
                   GENERATOR_FIELDS + generator_aux_fields(obj["fields"]["type"]))
    for link, group in enumerate(captured["particle_lists"]):
        prefix = f"particles.link[{link}]"
        state[f"{prefix}.count"] = {"t": "u", "v": len(group)}
        for index, obj in enumerate(group):
            add_object(f"{prefix}.particle[{index}]", obj, PARTICLE_FIELDS)
    for index, obj in enumerate(captured["appsrts"]):
        add_object(f"particles.appsrt[{index}]", obj, APPSRT_FIELDS)
    state["particles.pending_count"] = {"t": "u", "v": len(captured["pending_generators"])}
    for index, pending in enumerate(captured["pending_generators"]):
        reference(f"particles.pending[{index}].generator_index", pending["generator"], generator_ids)
    return {"frame": frame, "phase": "particles", "state": state}
