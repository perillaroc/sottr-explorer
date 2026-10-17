# -*- coding: utf-8 -*-
"""SX-019 research helper: independent readers for SOTTR archive containers.

Formats implemented from observed structure (cross-checked against two public
mod tools' readers; no third-party code copied):

- TIGER v5 archive (SOTTR): header {magic TAFS, version, numParts, numFiles, id,
  subId, platform[0x20]}, then numFiles x 32-byte descriptors:
      u64 nameHash, u64 locale, u32 size1, u32 size2, i16 part, u8 archiveId,
      u8 archiveSubId, u32 offset
  nameHash = FNV-1a 64 of the in-archive path (e.g. "pcx64-w\\globalgameflow.drm").

- CDRM container (compressed blob):
      magic 'CDRM', u32 type, u32 numChunks, u32 unknown(0)
      numChunks x {u32 uncompressedSize<<8, u32 compressedSize}
      chunk data at align16, each chunk padded to 16; when compressedSize ==
      uncompressedSize the chunk is raw, else skip 2 bytes then raw deflate.

- DRM v23 (resource collection file):
      i32 version(=23), then header fields:
        i32 includeLength, i32 dependenciesLength, i32 paddingLength,
        i32 size, i32 flags, i32 numResources, i32 mainResourceIndex,
        u64 locale
      numResources x identification (24 bytes):
        u32 bodySize, u8 type, u8 flags, i16 padding,
        u32 subTypeAndRefDefinitionsSize (subType=(v&0xFF)>>1, refDefs=v>>8),
        i32 id, u64 locale
      dependencies (dependenciesLength bytes): u64 locale + NUL-terminated path
      includes (includeLength bytes): same
      numResources x location (24 bytes):
        i32 uniqueKey ((type<<24)|id), i32 padding, i16 archivePart,
        u8 archiveId, u8 archiveSubId, u32 offset, u32 size,
        u32 offsetInBatch
      payload length = refDefinitionsSize + bodySize; if payload == size the blob
      is raw at offset, else it is a CDRM container.

- DTP resource (object graph):
      RefDefinitions header:
        i32 numInternalRefs, numWideExternalRefs, i32 numIntPatches,
        i32 numShortPatches, i32 numPackedExternalRefs
        internalRefs: numInternal x {i32 refOffset, i32 targetOffset}
        wideExternalRefs: numWide x {i32 refOffset, i32 resourceType,
                                     i32 resourceId, i32 resourceOffset}
        intPatches: numInt x i32
        shortPatches: numShort x u64
        packedExternalRefs: numPacked x u32 {refOffset:25 | type:7}
      body follows; body positions/offsets are body-relative.

Only read-only access to the locally installed game is performed.
"""

from __future__ import annotations

import struct
import zlib
from dataclasses import dataclass, field
from pathlib import Path
from typing import BinaryIO

FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
MASK64 = 0xFFFFFFFFFFFFFFFF

DEFAULT_LOCALE = 0xFFFFFFFFFFFFFFFF


def fnv1a64(text: str) -> int:
    value = FNV_OFFSET
    for ch in text:
        value ^= ord(ch)
        value = (value * FNV_PRIME) & MASK64
    return value


# ---------------------------------------------------------------- TIGER

@dataclass
class TigerEntry:
    name_hash: int
    locale: int
    size1: int
    size2: int
    part: int
    archive_id: int
    archive_sub_id: int
    offset: int
    source: str = ""


class TigerArchive:
    """One bigfile family (e.g. bigfile) with numbered parts .000-.NNN."""

    def __init__(self, base_path: Path):
        self.base_path = base_path
        self.parts: list[Path] = []
        with base_path.open("rb") as stream:
            raw = stream.read(0x38)
            magic, version, num_parts, num_files, archive_id = struct.unpack_from("<IIIII", raw, 0)
            if magic != 0x53464154:
                raise ValueError(f"bad magic {magic:08X} in {base_path}")
            if version != 5:
                raise ValueError(f"unexpected archive version {version} in {base_path}")
            self.version = version
            self.num_parts = num_parts
            self.archive_id = archive_id
            self.sub_id = struct.unpack_from("<i", raw, 0x14)[0]
            self.platform = raw[0x18:0x38].split(b"\0")[0].decode("ascii", "replace")
            table = stream.read(num_files * 32)
        self.num_files = num_files
        self.entries: list[TigerEntry] = []
        for i in range(num_files):
            values = struct.unpack_from("<QQIIhBBI", table, i * 32)
            self.entries.append(TigerEntry(*values, source=base_path.name))
        stem = base_path.name[: -len(".000.tiger")]
        for part in range(num_parts):
            candidate = base_path.with_name(f"{stem}.{part:03d}.tiger")
            self.parts.append(candidate)

    def find(self, name_or_hash: str | int) -> list[TigerEntry]:
        target = name_or_hash if isinstance(name_or_hash, int) else fnv1a64(name_or_hash)
        return [e for e in self.entries if e.name_hash == target]

    def read_part(self, part: int, offset: int, size: int) -> bytes:
        with self.parts[part].open("rb") as stream:
            stream.seek(offset)
            data = stream.read(size)
        if len(data) != size:
            raise IOError(f"short read in {self.parts[part]} at {offset}: {len(data)} != {size}")
        return data

    def read_file(self, entry: TigerEntry) -> bytes:
        return self.read_part(entry.part, entry.offset, entry.size1)


# ---------------------------------------------------------------- CDRM

def read_cdrm(blob: bytes) -> bytes:
    magic, kind, num_chunks, _unknown = struct.unpack_from("<IIII", blob, 0)
    if magic != 0x4D524443:
        raise ValueError("not a CDRM blob")
    chunks: list[tuple[int, int, int]] = []
    offset = 16 + 8 * num_chunks
    offset = (offset + 0xF) & ~0xF
    for i in range(num_chunks):
        uncompressed, compressed = struct.unpack_from("<II", blob, 16 + 8 * i)
        uncompressed >>= 8
        chunks.append((offset, uncompressed, compressed))
        offset = (offset + compressed + 0xF) & ~0xF
    out = bytearray()
    for chunk_offset, uncompressed, compressed in chunks:
        chunk = blob[chunk_offset:chunk_offset + compressed]
        if compressed == uncompressed:
            out += chunk[:uncompressed]
        else:
            # zlib-style 2-byte header followed by a raw deflate stream
            out += zlib.decompress(chunk[2:], -15, uncompressed)
    return bytes(out)


# ---------------------------------------------------------------- DRM

@dataclass
class DrmIdentification:
    body_size: int
    type: int
    flags: int
    sub_type: int
    ref_definitions_size: int
    id: int
    locale: int


@dataclass
class DrmLocation:
    unique_key: int
    part: int
    archive_id: int
    archive_sub_id: int
    offset: int
    size: int
    offset_in_batch: int

    @property
    def type(self) -> int:
        return (self.unique_key >> 24) & 0xFF

    @property
    def id(self) -> int:
        return self.unique_key & 0x00FFFFFF


@dataclass
class DrmFile:
    path: Path
    version: int
    locale: int
    main_resource_index: int
    identifications: list[DrmIdentification] = field(default_factory=list)
    locations: list[DrmLocation] = field(default_factory=list)
    dependencies: list[tuple[int, str]] = field(default_factory=list)
    includes: list[tuple[int, str]] = field(default_factory=list)

    def find(self, resource_type: int, resource_id: int) -> tuple[DrmIdentification, DrmLocation] | None:
        for ident, loc in zip(self.identifications, self.locations):
            if ident.type == resource_type and loc.id == resource_id:
                return ident, loc
        return None


def parse_drm(data: bytes, path: Path) -> DrmFile:
    version = struct.unpack_from("<i", data, 0)[0]
    if version != 23:
        raise ValueError(f"unsupported DRM version {version}")
    (include_length, dependencies_length, padding_length, size, flags,
     num_resources, main_resource_index) = struct.unpack_from("<iiiiiii", data, 4)
    locale = struct.unpack_from("<Q", data, 4 + 28)[0]
    pos = 4 + 36
    drm = DrmFile(path=path, version=version, locale=locale,
                  main_resource_index=main_resource_index)
    for i in range(num_resources):
        body_size, type_, flags_, _pad, subref, id_, loc = struct.unpack_from("<IBBhIiQ", data, pos + i * 24)
        drm.identifications.append(DrmIdentification(
            body_size=body_size, type=type_, flags=flags_,
            sub_type=(subref & 0xFF) >> 1, ref_definitions_size=subref >> 8,
            id=id_, locale=loc))
    pos += num_resources * 24

    def read_deps(pos: int, length: int):
        deps = []
        end = pos + length
        while pos < end:
            loc = struct.unpack_from("<Q", data, pos)[0]
            pos += 8
            text_end = data.index(b"\0", pos)
            deps.append((loc, data[pos:text_end].decode("utf-8", "replace")))
            pos = text_end + 1
        return deps, pos

    drm.dependencies, pos = read_deps(pos, dependencies_length)
    drm.includes, pos = read_deps(pos, include_length)
    for i in range(num_resources):
        unique, _pad, part, aid, asub, offset, size_, batch = struct.unpack_from("<iiHBBIII", data, pos + i * 24)
        drm.locations.append(DrmLocation(
            unique_key=unique, part=part, archive_id=aid, archive_sub_id=asub,
            offset=offset, size=size_, offset_in_batch=batch))
    return drm


def read_resource(archive: TigerArchive, drm: DrmFile, index: int) -> tuple[bytes, str]:
    """Return (payload, description) for the resource at `index`."""
    ident = drm.identifications[index]
    loc = drm.locations[index]
    payload_size = ident.ref_definitions_size + ident.body_size
    blob = archive.read_part(loc.part, loc.offset, loc.size)
    if payload_size == loc.size:
        return blob[:payload_size], "raw"
    return read_cdrm(blob)[:payload_size], "cdrm"


# archive location ids observed in resource location tables
ARCHIVE_FAMILIES = {
    1: "bigfile",
    10: "bigfile.update1.002",
    11: "bigfile.update2.002",
    12: "bigfile.update3.002",
}


class ArchiveRegistry:
    """Opens tiger families on demand and resolves resource locations to them."""

    def __init__(self, game_dir: Path):
        self.game_dir = game_dir
        self._cache: dict[str, TigerArchive] = {}

    def family(self, archive_id: int) -> TigerArchive:
        stem = ARCHIVE_FAMILIES.get(archive_id)
        if stem is None:
            raise KeyError(f"unknown archive id {archive_id}")
        if stem not in self._cache:
            self._cache[stem] = TigerArchive(self.game_dir / f"{stem}.000.tiger")
        return self._cache[stem]

    def read_resource(self, ident: DrmIdentification, loc: DrmLocation) -> tuple[bytes, str]:
        size = ident.ref_definitions_size + ident.body_size
        archive = self.family(loc.archive_id)
        blob = archive.read_part(loc.part, loc.offset, loc.size)
        if size == loc.size:
            return blob[:size], "raw"
        return read_cdrm(blob)[:size], f"cdrm/{archive.base_path.name}"


# ---------------------------------------------------------------- DTP

class DtpFile:
    """Parsed RefDefinitions header plus the resource body."""

    REF_INTERNAL = 1
    REF_WIDE_EXTERNAL = 2
    REF_PACKED_EXTERNAL = 3

    def __init__(self, payload: bytes):
        self.data = payload
        pos = 0
        (num_internal, num_wide, num_int_patches,
         num_short_patches, num_packed) = struct.unpack_from("<iiiii", payload, pos)
        pos += 20
        self.num_internal = num_internal
        self.num_wide = num_wide
        self.num_packed = num_packed
        self.internal_by_ref: dict[int, int] = {}
        for _ in range(num_internal):
            ref_offset, target_offset = struct.unpack_from("<ii", payload, pos)
            self.internal_by_ref[ref_offset] = target_offset
            pos += 8
        self.wide_by_ref: dict[int, tuple[int, int, int]] = {}
        for _ in range(num_wide):
            ref_offset, rtype, rid, roffset = struct.unpack_from("<iiii", payload, pos)
            # wide/packed refOffsets are in 32-bit words (see template: *4)
            self.wide_by_ref[ref_offset * 4] = (rtype, rid, roffset)
            pos += 16
        self.int_patches = list(struct.unpack_from(f"<{num_int_patches}i", payload, pos))
        pos += num_int_patches * 4
        for _ in range(num_short_patches):
            pos += 8
        self.packed_by_ref: dict[int, tuple[int, int]] = {}
        for _ in range(num_packed):
            value = struct.unpack_from("<I", payload, pos)[0]
            ref_offset = (value & 0x1FFFFFF) * 4
            rtype = value >> 25
            self.packed_by_ref[ref_offset] = (rtype, ref_offset)
            pos += 4
        self.body_pos = pos
        self.body = payload[self.body_pos:]
        self.ref_patch: dict[int, tuple[int, int]] = {}
        for ref_offset, target in self.internal_by_ref.items():
            self.ref_patch[self.body_pos + ref_offset] = (self.REF_INTERNAL, self.body_pos + target)
        for ref_offset, (rtype, rid, roffset) in self.wide_by_ref.items():
            self.ref_patch[self.body_pos + ref_offset] = (self.REF_WIDE_EXTERNAL, rtype)
        for ref_offset, (rtype, _ro) in self.packed_by_ref.items():
            self.ref_patch[self.body_pos + ref_offset] = (self.REF_PACKED_EXTERNAL, rtype)

    # -- primitive readers -------------------------------------------------

    def u8(self, pos: int) -> int:
        return self.data[pos]

    def u16(self, pos: int) -> int:
        return struct.unpack_from("<H", self.data, pos)[0]

    def u32(self, pos: int) -> int:
        return struct.unpack_from("<I", self.data, pos)[0]

    def i32(self, pos: int) -> int:
        return struct.unpack_from("<i", self.data, pos)[0]

    def u64(self, pos: int) -> int:
        return struct.unpack_from("<Q", self.data, pos)[0]

    def f32(self, pos: int) -> float:
        return struct.unpack_from("<f", self.data, pos)[0]

    def string_at(self, pos: int) -> str:
        end = self.data.index(b"\0", pos)
        return self.data[pos:end].decode("utf-8", "replace")

    def ref_target(self, pos: int) -> int | None:
        """Internal-ref target body offset for the ref stored at pos, else None."""
        entry = self.ref_patch.get(pos)
        if entry is None or entry[0] != self.REF_INTERNAL:
            return None
        return entry[1]

    def ref_kind(self, pos: int) -> int:
        entry = self.ref_patch.get(pos)
        return entry[0] if entry else 0
