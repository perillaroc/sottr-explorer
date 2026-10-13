#!/usr/bin/env python3
"""Read-only, offline Shadow of the Tomb Raider save observations (Python 3.10+)."""
from __future__ import annotations

import argparse
import ctypes
import csv
import io
import research_decoder
from datetime import datetime, timedelta, timezone
import hashlib
import itertools
import json
import math
import os
from pathlib import Path
import re
import struct
import sys
from dataclasses import asdict, dataclass, field
from enum import Enum
from typing import Any
import zlib

VERSION = "0.2.0"
SAVE_NAME = re.compile(r"(?:profile|save\d+)\.dat\Z", re.IGNORECASE)
MAX_COMPRESSED = 64 * 1024 * 1024
MAX_EXPANDED = 64 * 1024 * 1024
MAX_FILES = 128
MAX_PAIRS = 64


class CollectibleState(str, Enum):
    UNKNOWN = "unknown"
    COLLECTED = "collected"
    UNCOLLECTED = "uncollected"


@dataclass
class Evidence:
    status: str = "unsupported"
    method: str = "No validated save-field mapping"
    sources: list[str] = field(default_factory=list)

    def __post_init__(self):
        if self.status not in {"observed", "candidate", "validated", "unsupported"}:
            raise ValueError("Invalid evidence status")
        if self.status == "validated" and not self.sources:
            raise ValueError("Validated evidence requires sources")


def validate_count(value: int | None):
    if value is not None and (type(value) is not int or value < 0):
        raise ValueError("Count must be a nonnegative integer or unknown (None)")


def validate_progress(collected: int | None, total: int | None, evidence: Evidence):
    validate_count(collected)
    validate_count(total)
    if collected is not None and total is not None and collected > total:
        raise ValueError("Collected count exceeds total")
    if (collected is not None or total is not None) and evidence.status != "validated":
        raise ValueError("Known progress counts require validated evidence")


@dataclass
class CollectibleItem:
    item_id: str
    name: str | None = None
    state: CollectibleState = CollectibleState.UNKNOWN
    evidence: Evidence = field(default_factory=Evidence)

    def __post_init__(self):
        if not self.item_id:
            raise ValueError("Item identifier is required")
        self.state = CollectibleState(self.state)
        if self.state != CollectibleState.UNKNOWN and self.evidence.status != "validated":
            raise ValueError("Known collectible state requires validated evidence")


@dataclass
class CollectibleCategory:
    category_id: str
    items: list[CollectibleItem] = field(default_factory=list)
    collected: int | None = None
    total: int | None = None
    evidence: Evidence = field(default_factory=Evidence)

    def __post_init__(self):
        validate_progress(self.collected, self.total, self.evidence)


@dataclass
class RegionProgress:
    region_id: str
    categories: list[CollectibleCategory] = field(default_factory=list)
    completion_percent: float | None = None
    evidence: Evidence = field(default_factory=Evidence)

    def __post_init__(self):
        validate_percent(self.completion_percent, self.evidence)


def validate_percent(value: float | None, evidence: Evidence):
    if value is not None:
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or not 0 <= value <= 100:
            raise ValueError("Completion percentage must be finite and in [0, 100]")
        if evidence.status != "validated":
            raise ValueError("Known completion requires validated evidence")


@dataclass
class SaveSlot:
    source_file: str
    role: str | None = None
    overall_completion_percent: float | None = None
    regions: list[RegionProgress] | None = None
    collectible_categories: list[CollectibleCategory] | None = None
    evidence: Evidence = field(default_factory=Evidence)

    def __post_init__(self):
        validate_percent(self.overall_completion_percent, self.evidence)
        if self.role is not None and self.evidence.status != "validated":
            raise ValueError("Save slot role requires validated evidence")


@dataclass
class SaveProfile:
    source_file: str | None
    slots: list[SaveSlot] = field(default_factory=list)
    evidence: Evidence = field(default_factory=Evidence)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def file_hash(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def bounded_decompress(data: bytes, maximum: int = MAX_EXPANDED) -> tuple[dict[str, Any], bytes | None]:
    """Accept only a complete first zlib stream at offset zero; retain trailing metadata."""
    if maximum < 1:
        raise ValueError("Decompression limit must be positive")
    state: dict[str, Any] = {"format": "zlib", "stream_offset": 0,
                             "valid": False, "eof": False, "trailing_bytes": None,
                             "error": None, "maximum_expanded_bytes": maximum}
    decoder = zlib.decompressobj()
    try:
        # max_length + 1 detects bombs without allocating their full expansion.
        raw = decoder.decompress(data, maximum + 1)
        state["eof"] = decoder.eof
        if len(raw) > maximum or decoder.unconsumed_tail:
            state["error"] = "expanded_size_limit_exceeded"
            return state, None
        if not decoder.eof:
            state["error"] = "truncated_or_incomplete_zlib_stream"
            return state, None
        trailing = decoder.unused_data
        state.update(valid=True, trailing_bytes=len(trailing),
                     trailing_sha256=sha256(trailing) if trailing else None,
                     trailing_prefix_hex=trailing[:32].hex(),
                     compressed_stream_bytes=len(data) - len(trailing),
                     warning="Trailing bytes are not interpreted" if trailing else None)
        return state, raw
    except zlib.error as exc:
        state["error"] = "invalid_zlib_stream"
        state["detail"] = str(exc)
        return state, None


def ascii_strings(data: bytes, limit: int = 2000) -> dict[str, Any]:
    results = []
    total = 0
    for match in re.finditer(rb"[\x20-\x7e]{5,}", data):
        total += 1
        if len(results) < limit:
            value = match.group()
            results.append({"offset": match.start(), "length": len(value),
                            "text": value[:256].decode("ascii"), "text_truncated": len(value) > 256})
    return {"minimum_length": 5, "total_matches": total, "listed": results,
            "list_truncated": total > limit, "interpretation": "Printable byte runs, not validated text fields"}


def numeric_candidates(data: bytes, offset: int) -> dict[str, Any]:
    results: dict[str, Any] = {"offset": offset, "status": "candidate", "semantics": None}
    for name, fmt in (("u16_le", "<H"), ("u16_be", ">H"), ("u32_le", "<I"),
                      ("u32_be", ">I"), ("i32_le", "<i"), ("u64_le", "<Q"),
                      ("f32_le", "<f"), ("f64_le", "<d")):
        size = struct.calcsize(fmt)
        if offset + size <= len(data):
            value = struct.unpack_from(fmt, data, offset)[0]
            results[name] = value if not isinstance(value, float) or math.isfinite(value) else None
    return results


def compare_bytes(a: bytes, b: bytes, max_runs: int = 2000, max_candidates: int = 32) -> dict[str, Any]:
    common = min(len(a), len(b))
    runs = []
    run_start = None
    run_count = 0
    changed = 0
    bits = 0
    bit_positions = [0] * 8
    pages: dict[int, int] = {}
    candidate_offsets = []
    for offset in range(max(len(a), len(b))):
        different = offset >= common or a[offset] != b[offset]
        if different:
            changed += 1
            pages[offset // 4096] = pages.get(offset // 4096, 0) + 1
            if offset < common:
                xor = a[offset] ^ b[offset]
                bits += xor.bit_count()
                for bit in range(8):
                    bit_positions[bit] += (xor >> bit) & 1
            if run_start is None:
                run_start = offset
                if len(candidate_offsets) < max_candidates and offset < common:
                    candidate_offsets.append(offset)
        if run_start is not None and (not different or offset == max(len(a), len(b)) - 1):
            end = offset + 1 if different else offset
            run_count += 1
            if len(runs) < max_runs:
                runs.append({"start": run_start, "end_exclusive": end, "length": end - run_start})
            run_start = None
    return {"length_a": len(a), "length_b": len(b), "common_bytes": common,
            "changed_bytes_including_length_difference": changed,
            "changed_bytes_in_common_range": changed - abs(len(a) - len(b)),
            "changed_bits_in_common_range": bits, "changed_bit_positions_lsb_first": bit_positions,
            "length_difference_bytes": abs(len(a) - len(b)), "contiguous_run_count": run_count,
            "contiguous_runs": runs, "runs_truncated": run_count > max_runs,
            "page_size": 4096, "changed_bytes_per_page": [{"page": p, "offset": p * 4096, "changed_bytes": n} for p, n in sorted(pages.items())],
            "candidate_sampling": "First changed run starts, maximum %d; numeric interpretations have no established semantics" % max_candidates,
            "numeric_candidates": [{"a": numeric_candidates(a, off), "b": numeric_candidates(b, off)} for off in candidate_offsets]}


def windows_documents() -> Path | None:
    if os.name != "nt":
        return None
    # SHGetKnownFolderPath(FOLDERID_Documents), supports redirected Documents.
    import uuid
    class GUID(ctypes.Structure):
        _fields_ = [("Data1", ctypes.c_uint32), ("Data2", ctypes.c_uint16),
                    ("Data3", ctypes.c_uint16), ("Data4", ctypes.c_ubyte * 8)]
    guid = GUID.from_buffer_copy(uuid.UUID("FDD39AD0-238F-46AF-ADB4-6C85480369C7").bytes_le)
    result = ctypes.c_wchar_p()
    shell = ctypes.windll.shell32
    shell.SHGetKnownFolderPath.argtypes = [ctypes.POINTER(GUID), ctypes.c_uint32, ctypes.c_void_p, ctypes.POINTER(ctypes.c_wchar_p)]
    shell.SHGetKnownFolderPath.restype = ctypes.c_long
    if shell.SHGetKnownFolderPath(ctypes.byref(guid), 0, None, ctypes.byref(result)) != 0:
        return None
    try:
        return Path(result.value)
    finally:
        ctypes.windll.ole32.CoTaskMemFree.argtypes = [ctypes.c_void_p]
        ctypes.windll.ole32.CoTaskMemFree(ctypes.cast(result, ctypes.c_void_p))


def discover_roots() -> list[Path]:
    documents = [Path.home() / "Documents"]
    known = windows_documents()
    if known:
        documents.insert(0, known)
    for variable in ("OneDrive", "OneDriveConsumer", "OneDriveCommercial"):
        if os.environ.get(variable):
            documents.append(Path(os.environ[variable]) / "Documents")
    roots = []
    for document in documents:
        game = document / "Shadow of the Tomb Raider"
        if game.is_dir() and not game.is_symlink():
            resolved = game.resolve()
            if resolved not in roots:
                roots.append(resolved)
    return roots


def enumerate_saves(roots: list[Path]) -> list[Path]:
    result = set()
    for root in roots:
        for directory, dirs, files in os.walk(root, followlinks=False):
            current = Path(directory)
            depth = len(current.relative_to(root).parts)
            dirs[:] = sorted(d for d in dirs if depth < 4 and not (current / d).is_symlink())
            for name in sorted(files):
                candidate = current / name
                if SAVE_NAME.fullmatch(name) and candidate.is_file() and not candidate.is_symlink():
                    result.add(candidate.resolve())
                    if len(result) > MAX_FILES:
                        raise ValueError(f"More than {MAX_FILES} save files; select a narrower input directory")
    return sorted(result)


def validate_output(output: Path, roots: list[Path]):
    resolved = output.resolve()
    for root in roots:
        root = root.resolve()
        if resolved == root or root in resolved.parents or resolved in root.parents:
            raise ValueError("Output must be separate from every input directory (no parent/child overlap)")
    if output.exists():
        raise ValueError("Output directory already exists; choose a new directory to avoid overwriting results")


def duplicate_groups(records: list[dict[str, Any]], field_name: str) -> list[list[str]]:
    grouped: dict[str, list[str]] = {}
    for record in records:
        key = record.get(field_name)
        if key is not None:
            grouped.setdefault(key, []).append(record["file"])
    return [group for group in grouped.values() if len(group) > 1]


def load_catalog(path: Path) -> dict[str, Any]:
    if path.stat().st_size > 4 * 1024 * 1024:
        raise ValueError("Catalog exceeds 4 MiB limit")
    content = path.read_bytes()
    def unique_object(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"Duplicate JSON key in catalog: {key}")
            result[key] = value
        return result

    def reject_constant(value):
        raise ValueError(f"Non-standard JSON numeric constant in catalog: {value}")

    document = json.loads(content, object_pairs_hook=unique_object, parse_constant=reject_constant)
    if not isinstance(document, dict):
        raise ValueError("Catalog must be a JSON object")
    if document.get("source") is not None and not isinstance(document["source"], str):
        raise ValueError("Catalog source must be a string or null")
    mapping = document.get("mapping")
    if not isinstance(mapping, dict) or len(mapping) > 10000:
        raise ValueError("Catalog requires a mapping object with at most 10000 IDs")
    normalized = {}
    for identifier, names in mapping.items():
        value = int(identifier, 0)
        if not 0 <= value <= 0xffffffff or not isinstance(names, list) or not names or any(not isinstance(n, str) or len(n) > 1000 for n in names):
            raise ValueError("Invalid catalog ID or name list")
        # Keep multiple names per ID, including repeated names, without silently overwriting.
        normalized.setdefault(value, []).extend(names)
    return {"source": document.get("source"), "license": document.get("license"), "sha256": sha256(content), "mapping": normalized}


def scan_catalog(raw: bytes, catalog: dict[str, Any], limit: int = 20000) -> dict[str, Any]:
    mapping = catalog["mapping"]
    matches = []
    totals = {"little": 0, "big": 0}
    unique = {"little": set(), "big": set()}
    for byteorder in ("little", "big"):
        for identifier, names in mapping.items():
            pattern = identifier.to_bytes(4, byteorder)
            position = raw.find(pattern)
            while position >= 0:
                totals[byteorder] += 1
                unique[byteorder].add(identifier)
                if len(matches) < limit:
                    following = raw[position + 4:position + 8]
                    matches.append({"offset": position, "byteorder": byteorder, "id": f"0x{identifier:08X}",
                                    "candidate_names": names, "following_word_u32_le": int.from_bytes(following, "little") if len(following) == 4 else None,
                                    "following_word_i32_le": int.from_bytes(following, "little", signed=True) if len(following) == 4 else None,
                                    "collectible_state": "unknown"})
                position = raw.find(pattern, position + 1)
    tables = []
    # This location is an observation from the supplied sample family, NOT a general file schema.
    start, count, stride = 0x44, 512, 8
    if len(raw) >= start + count * stride:
        records = []
        for index in range(count):
            offset = start + index * stride
            identifier, word = struct.unpack_from("<II", raw, offset)
            if identifier not in mapping:
                break
            records.append({"index": index, "id_offset": offset, "following_word_offset": offset + 4,
                            "id": f"0x{identifier:08X}", "candidate_names": mapping[identifier],
                            "raw_following_word": word, "collectible_state": "unknown"})
        if len(records) == count:
            block = raw[start:start + count * stride]
            copies = []
            position = raw.find(block)
            while position >= 0:
                copies.append(position)
                position = raw.find(block, position + 1)
            tables.append({"status": "candidate", "basis": "512 consecutive catalog IDs at the sample-family offset, each followed by an uninterpreted word",
                           "offset": start, "stride": stride, "record_count": count, "exact_block_copy_offsets": copies,
                           "block_sha256": sha256(block), "records": records})
    return {"status": "candidate", "source": catalog["source"], "catalog_sha256": catalog["sha256"],
            "warning": "ID occurrence and following words do not establish collectible ownership, availability, or completion",
            "total_occurrences": totals, "unique_ids": {key: len(value) for key, value in unique.items()},
            "occurrences": sorted(matches, key=lambda m: (m["offset"], m["byteorder"])),
            "occurrences_truncated": sum(totals.values()) > limit, "candidate_tables": tables}


def read_compressed(path: Path) -> bytes:
    with path.open("rb") as stream:
        data = stream.read(MAX_COMPRESSED + 1)
    if len(data) > MAX_COMPRESSED:
        raise ValueError("Input exceeds compressed size limit or changed during analysis")
    return data


def run_analysis(roots: list[Path], output: Path, maximum: int = MAX_EXPANDED,
                 pair_limit: int = MAX_PAIRS, catalog: dict[str, Any] | None = None) -> dict[str, Any]:
    if maximum < 1 or maximum > MAX_EXPANDED or not 0 <= pair_limit <= MAX_PAIRS:
        raise ValueError("Resource limits exceed supported bounds")
    validate_output(output, roots)
    paths = enumerate_saves(roots)
    if not paths:
        raise ValueError("No profile.dat or save<number>.dat found within input depth 4")
    before = {str(path): file_hash(path) for path in paths}
    records = []
    logs = [f"sottr-inspect {VERSION}; offline, read-only", "All offsets are decimal, zero-based decompressed offsets unless stated otherwise."]
    raw_by_path = {}
    profiles = []
    for directory in sorted({path.parent for path in paths}):
        profile_files = [p for p in paths if p.parent == directory and p.name.lower() == "profile.dat"]
        profiles.append(SaveProfile(str(profile_files[0]) if profile_files else None,
                        [SaveSlot(str(p)) for p in paths if p.parent == directory and p.name.lower() != "profile.dat"]))
    for path in paths:
        size = path.stat().st_size
        record: dict[str, Any] = {"file": str(path), "name": path.name, "compressed_bytes": size,
                                  "compressed_sha256": before[str(path)], "uncompressed_sha256": None}
        if size > MAX_COMPRESSED:
            record["decompression"] = {"valid": False, "error": "compressed_size_limit_exceeded"}
            records.append(record)
            continue
        with path.open("rb") as stream:
            data = stream.read(MAX_COMPRESSED + 1)
        if len(data) > MAX_COMPRESSED or sha256(data) != before[str(path)]:
            raise ValueError(f"Input changed while reading: {path}")
        state, raw = bounded_decompress(data, maximum)
        record["decompression"] = state
        if raw is not None:
            record.update(uncompressed_bytes=len(raw), uncompressed_sha256=sha256(raw),
                          zero_bytes=raw.count(0), prefix_hex=raw[:64].hex(), ascii_strings=ascii_strings(raw))
            # Only keep hashes/metadata across files. Pairs are re-read below to bound total RAM.
            raw_by_path[path] = record["uncompressed_sha256"]
            record["candidate_header_numbers"] = [numeric_candidates(raw, offset) for offset in (0, 8, 0x28, 0x11e0, 0x1200) if offset < len(raw)]
            if len(raw) >= 8:
                ticks = struct.unpack_from("<Q", raw)[0]
                try:
                    value = (datetime(1601, 1, 1, tzinfo=timezone.utc) + timedelta(microseconds=ticks // 10)).isoformat()
                except OverflowError:
                    value = None
                record["candidate_header_filetime"] = {"status": "candidate", "offset": 0, "raw_u64_le": ticks,
                    "utc_interpretation": value, "submicrosecond_100ns_remainder": ticks % 10,
                    "semantics": None, "warning": "FILETIME numeric interpretation only; actual timestamp purpose is unverified"}
            if path.name.lower() != "profile.dat":
                record["candidate_packed_stream"] = research_decoder.packed_sections(raw)
            if catalog:
                record["catalog_candidate_scan"] = scan_catalog(raw, catalog)
                if path.name.lower() != "profile.dat":
                    record["present_item_id_inventory"] = research_decoder.prefix_inventory(raw, catalog)
                    item_inventory = record["present_item_id_inventory"]
                    overflow = research_decoder.overflow_inventory(raw, catalog, record["candidate_packed_stream"])
                    item_inventory["prefix_nonzero_records"] = item_inventory.pop("nonzero_records", 0)
                    item_inventory["prefix_catalog_matched_records"] = item_inventory.pop("catalog_matched_records", 0)
                    item_inventory["prefix_capacity"] = item_inventory.pop("capacity", 512)
                    item_inventory["overflow_nonzero_records"] = len(overflow)
                    item_inventory["records"].extend(overflow)
                    item_inventory["total_nonzero_records"] = len(item_inventory["records"])
                    item_inventory["unique_id_count"] = len({item["id"] for item in item_inventory["records"]})
                    item_inventory["duplicate_id_record_count"] = item_inventory["total_nonzero_records"] - item_inventory["unique_id_count"]
                    item_inventory["total_catalog_matched_records"] = sum(bool(x["candidate_names"]) for x in item_inventory["records"])
                    item_inventory["record_sources"] = ["candidate_prefix", "candidate_type28_overflow"]
                    record["reference_presence_differences"] = research_decoder.reference_differences(item_inventory, catalog)
            if path.name.lower() != "profile.dat":
                record["derived_candidates"] = {"overall_completion": research_decoder.overall_candidate(raw),
                                                "region_counters": research_decoder.region_candidates(raw, record["candidate_packed_stream"])}
        records.append(record)
        logs.append(f"{path.name}: compressed={size}, zlib_valid={state['valid']}, error={state['error']}, expanded={record.get('uncompressed_bytes')}, trailing={state.get('trailing_bytes')}")
    pairs = []
    eligible = [p for p in raw_by_path if p.name.lower() != "profile.dat"]
    possible_pairs = len(eligible) * (len(eligible) - 1) // 2
    for a, b in itertools.islice(itertools.combinations(eligible, 2), pair_limit):
        _, raw_a = bounded_decompress(read_compressed(a), maximum)
        _, raw_b = bounded_decompress(read_compressed(b), maximum)
        if raw_a is None or raw_b is None or sha256(raw_a) != raw_by_path[a] or sha256(raw_b) != raw_by_path[b]:
            raise ValueError("Input changed during pair comparison; no report written")
        result = compare_bytes(raw_a, raw_b)
        pairs.append({"file_a": str(a), "file_b": str(b), **result})
        logs.append(f"PAIR {a.name} / {b.name}: changed_bytes={result['changed_bytes_including_length_difference']}, changed_bits={result['changed_bits_in_common_range']}, runs={result['contiguous_run_count']}")
    after = {str(path): file_hash(path) for path in paths}
    if before != after:
        raise ValueError("Input hashes changed during analysis; close the game and retry with copied saves")
    report = {"schema_version": 2, "tool_version": VERSION,
              "scope": "Read-only observations with explicitly separate, unvalidated community-correlated candidates",
              "known_observations": {"inventory": records, "duplicate_compressed_groups": duplicate_groups(records, "compressed_sha256"),
                                     "duplicate_uncompressed_groups": duplicate_groups(records, "uncompressed_sha256"),
                                     "pairwise_comparisons": pairs, "possible_save_pairs": possible_pairs,
                                     "pairs_analyzed": len(pairs), "pairs_truncated": possible_pairs > pair_limit},
              "candidate_interpretations": {"location": "pairwise_comparisons[].numeric_candidates", "validated_semantics": None},
              "unsupported": {"overall_progress": None, "region_progress": None, "item_progress": None,
                              "reason": "No controlled in-game before/after evidence or validated schema maps bytes to progress"},
              "profiles": [asdict(profile) for profile in profiles],
              "integrity": {"all_input_hashes_unchanged": True, "before": before, "after": after},
              "limits": {"compressed_bytes_per_file": MAX_COMPRESSED, "expanded_bytes_per_file": maximum,
                         "files": MAX_FILES, "pairs": pair_limit, "ascii_strings_per_file": 2000,
                         "diff_runs_per_pair": 2000, "numeric_candidate_offsets_per_pair": 32}}
    errors = sum(not r["decompression"]["valid"] for r in records)
    summary = ["Shadow of the Tomb Raider 只读存档研究", "", f"文件：{len(records)}；成功解压：{len(records) - errors}；解压失败：{errors}",
               f"存档两两比较：{len(pairs)} / {possible_pairs}", f"输入文件分析前后 SHA-256 相同：是", "", "已验证模型层：整体百分比为 null、收藏品状态为 unknown", "下方另列社区证据支持的候选结果；存档槽位用途尚未验证", "字节不同或数值相似不能证明任务或收藏品状态。", "", "详细数据：report.json", "复现参数：invocation.json", "观察日志：research.log"]
    present_items = []
    for record in records:
        for item in record.get("present_item_id_inventory", {}).get("records", []):
            present_items.append({"file": record["file"], **item})
    item_csv = io.StringIO(newline="")
    columns = ["file", "table", "index", "id_offset", "id", "raw_low_two_bits", "internal_names", "raw_companion_u32", "raw_companion_hex", "presence_only", "collectible_state", "region_inference", "source", "license"]
    writer = csv.DictWriter(item_csv, fieldnames=columns)
    writer.writeheader()
    for item in present_items:
        writer.writerow({"file": item["file"], "table": item["table"], "raw_low_two_bits": item["raw_low_two_bits"], "index": item["index"], "id_offset": item["id_offset"], "id": item["id"],
                         "internal_names": json.dumps(item["candidate_names"], ensure_ascii=False),
                         "raw_companion_u32": item["raw_following_word"], "raw_companion_hex": item["raw_following_word_hex"],
                         "presence_only": True, "collectible_state": "unknown", "region_inference": "unknown",
                         "source": item["source"], "license": item["license"]})
    reference_rows = []
    for record in records:
        for group in record.get("reference_presence_differences", {}).get("groups", []):
            for item in group["reference_present_user_absent"]:
                reference_rows.append({"file": record["file"], "internal_name_prefix": group["internal_name_prefix"],
                                       **item, "status": "community_correlated", "meaning": "reference_present_user_absent"})
    reference_csv = io.StringIO(newline="")
    reference_writer = csv.DictWriter(reference_csv, fieldnames=["file", "internal_name_prefix", "id", "candidate_names", "status", "meaning", "collectible_state", "region_inference"])
    reference_writer.writeheader()
    for row in reference_rows:
        reference_writer.writerow({**row, "candidate_names": json.dumps(row["candidate_names"], ensure_ascii=False)})
    summary.extend(["", "v0.2 候选解释与真实游戏状态分开：", "物品 ID 清单：present_item_ids.json / present_item_ids.csv（仅表示候选前缀或有效声明流的溢出记录中存在）"])
    for record in records:
        value = record.get("derived_candidates", {}).get("overall_completion", {}).get("candidate_percent")
        if value is not None:
            summary.append(f"{record['name']}：整体百分比候选 {value:.6f}%（community_correlated，未经游戏内独立核实）")
    summary.extend(["", "参考清单差异（只比较两份公开 100% 参考存档共有的有限分类，不是通用全收集结论）："])
    for record in records:
        groups = record.get("reference_presence_differences", {}).get("groups", [])
        if groups:
            counts = {g["internal_name_prefix"]: g["absent_reference_count"] for g in groups}
            caches = counts.get("CollectibleTypeSurvivalCache_", 0) + counts.get("RareResourceCacheCollectible_", 0)
            monoliths = counts.get("CollectibleTypeMonolith_", 0)
            crypts = counts.get("CollectibleTypeCryptTreasure_", 0)
            summary.append(f"{record['name']}：参考中有、当前活动记录中未见 {caches + monoliths + crypts} 项（贮藏点 {caches}、独石 {monoliths}、地下墓穴宝藏 {crypts}）")
    summary.extend(["逐项 ID 与内部名：reference_presence_differences.csv / reference_presence_differences.json", "当前出现记录：present_item_ids.csv / present_item_ids.json；状态和地区仍为 unknown"])
    output.mkdir(parents=True, exist_ok=False)
    for name, content in (("reference_presence_differences.csv", reference_csv.getvalue()),
                          ("reference_presence_differences.json", json.dumps({"warning": "Reference-only presence differences; not a universal missing list. See report.json for sources and coverage.", "records": reference_rows}, ensure_ascii=False, indent=2)),
                          ("present_item_ids.json", json.dumps({"warning": research_decoder.WARNING, "records": present_items}, ensure_ascii=False, indent=2)),
                          ("present_item_ids.csv", item_csv.getvalue()), ("report.json", json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False)),
                          ("summary.txt", "\n".join(summary) + "\n"), ("research.log", "\n".join(logs) + "\n"),
                          ("invocation.json", json.dumps({"tool_version": VERSION, "inputs": [str(r) for r in roots],
                              "output": str(output), "maximum_expanded_bytes": maximum, "pair_limit": pair_limit, "catalog_sha256": catalog["sha256"] if catalog else None}, ensure_ascii=False, indent=2))):
        with (output / name).open("x", encoding="utf-8") as stream:
            stream.write(content)
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Offline read-only SOTTR save observations. This does not decode game progress.")
    parser.add_argument("--input", type=Path, action="append", help="Input directory; repeat for multiple roots. Omit for Windows Documents/OneDrive discovery.")
    parser.add_argument("--output", type=Path, required=True, help="New output directory separate from inputs; existing directories are rejected.")
    parser.add_argument("--max-expanded-mib", type=int, default=64, choices=range(1, 65), metavar="1..64")
    parser.add_argument("--max-pairs", type=int, default=64, choices=range(65), metavar="0..64")
    parser.add_argument("--catalog", type=Path, help="Optional local JSON ID catalog; scan occurrences without inferring ownership.")
    parser.add_argument("--version", action="version", version=VERSION)
    args = parser.parse_args(argv)
    try:
        roots = [root.resolve(strict=True) for root in args.input] if args.input else discover_roots()
        if not roots or any(not root.is_dir() for root in roots):
            raise ValueError("No valid input directory; pass --input explicitly")
        # Deduplicate overlapping roots to avoid representing files twice.
        roots = sorted(set(roots))
        roots = [root for root in roots if not any(other in root.parents for other in roots)]
        report = run_analysis(roots, args.output, args.max_expanded_mib * 1024 * 1024, args.max_pairs, load_catalog(args.catalog) if args.catalog else None)
        print(f"Report: {args.output / 'report.json'}")
        print("Input hashes unchanged. See separately labeled community-correlated candidates; validated model fields remain unknown.")
        return 2 if any(not r["decompression"]["valid"] for r in report["known_observations"]["inventory"]) else 0
    except (OSError, ValueError) as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
