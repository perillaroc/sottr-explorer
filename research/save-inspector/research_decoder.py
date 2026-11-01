"""Independently implemented read-only, sample-family candidate interpretations.
No authoritative item-state decoder. Third-party source code is not incorporated.
"""
from __future__ import annotations
import math
import struct
from typing import Any

PUBLIC_SOURCE = 'https://tombraiders.net/stella/savegame/TR11saves.html'
WARNING = ('Research interpretation only; not independently validated against the game UI. '
           'ID presence is not ownership, and absent IDs are not a missing-item list.')


def prefix_inventory(raw: bytes, catalog: dict[str, Any]) -> dict[str, Any]:
    """Observe the fixed candidate prefix, including unknown IDs and empty slots."""
    start, slots, stride = 0x44, 512, 8
    if len(raw) != 4194304 or not packed_sections(raw).get('valid'):
        return {'status': 'unsupported', 'reason': 'Candidate sample-family stream guard did not pass', 'records': []}
    records = []
    empty = 0
    for index in range(slots):
        offset = start + stride * index
        identifier, companion = struct.unpack_from('<II', raw, offset)
        if identifier == 0:
            empty += 1
            continue
        records.append({'index': index, 'id_offset': offset, 'following_word_offset': offset + 4,
                        'id': f'0x{identifier:08X}', 'candidate_names': catalog['mapping'].get(identifier, []),
                        'raw_following_word': companion, 'raw_following_word_hex': f'0x{companion:08X}',
                        'table': 'candidate_prefix', 'raw_low_two_bits': companion & 3,
                        'presence_only': True, 'collectible_state': 'unknown', 'region_inference': 'unknown',
                        'source': catalog.get('source'), 'license': catalog.get('license', 'Not specified')})
    return {'status': 'candidate', 'basis': 'Observed fixed prefix in private and public 4 MiB sample families; not a universal format signature',
            'offset': start, 'stride': stride, 'capacity': slots, 'zero_id_slots': empty,
            'nonzero_records': len(records), 'catalog_matched_records': sum(bool(x['candidate_names']) for x in records),
            'warning': WARNING, 'records': records}


def overall_candidate(raw: bytes) -> dict[str, Any]:
    if len(raw) != 4194304 or not packed_sections(raw).get('valid'):
        return {'status': 'unsupported', 'candidate_percent': None, 'reason': 'Candidate sample-family stream guard did not pass'}
    value = struct.unpack_from('<f', raw, 0x28)[0]
    sensible = math.isfinite(value) and 0 <= value <= 100
    return {'status': 'community_correlated' if sensible else 'unsupported', 'offset': 0x28, 'encoding': 'float32_le',
            'raw_hex': raw[0x28:0x2c].hex(), 'candidate_percent': value if sensible else None,
            'source': PUBLIC_SOURCE, 'basis': 'Public author-labeled 100% saves have 100.0 here; early Cozumel saves have lower increasing values',
            'warning': 'Raw-only API requires caller-provided campaign-save context; structural guard cannot distinguish challenge tomb or renamed files. Whole-archive labels do not guarantee every backup is 100%; no controlled UI corroboration or completion formula. ' + WARNING}


def packed_sections(raw: bytes) -> dict[str, Any]:
    """Bounded experimental walker: all offsets derived from the current input."""
    start = 0xBFD4
    if len(raw) != 4194304 or len(raw) < start:
        return {'status': 'unsupported', 'valid': False, 'sections': [], 'reason': 'Unrecognized sample-family size'}
    length = struct.unpack_from('<I', raw, start - 4)[0]
    end = start + length
    result = {'status': 'candidate', 'valid': False, 'length_offset': start - 4, 'start': start,
              'byte_length': length, 'end_exclusive': end, 'sections': [],
              'basis': 'Empirical low-8-bit type and high-24-bit size in DWORDs, including the header'}
    if length == 0 or length % 4 or end > len(raw):
        result['reason'] = 'Invalid stream length'
        return result
    cursor = start
    while cursor < end:
        if len(result['sections']) >= 50000 or cursor + 4 > end:
            result['reason'] = 'Section limit or truncated header'
            return result
        header = struct.unpack_from('<I', raw, cursor)[0]
        size = (header >> 8) * 4
        if size < 4 or cursor + size > end:
            result['reason'] = 'Invalid or out-of-bounds section size'
            return result
        result['sections'].append({'offset': cursor, 'type': header & 255, 'dwords': header >> 8, 'bytes': size})
        cursor += size
    result['valid'] = cursor == end
    result['section_count'] = len(result['sections'])
    return result


def overflow_inventory(raw: bytes, catalog: dict[str, Any], stream: dict[str, Any]) -> list[dict[str, Any]]:
    records = []
    if not stream.get('valid'):
        return records
    for section in stream['sections']:
        if section['type'] != 28 or section['bytes'] != 4112:
            continue
        for index in range(512):
            offset = section['offset'] + 16 + index * 8
            identifier, word = struct.unpack_from('<II', raw, offset)
            if identifier == 0:
                continue
            records.append({'index': index, 'id_offset': offset, 'following_word_offset': offset + 4,
                            'table': 'candidate_type28_overflow', 'id': f'0x{identifier:08X}',
                            'candidate_names': catalog['mapping'].get(identifier, []),
                            'raw_following_word': word, 'raw_following_word_hex': f'0x{word:08X}',
                            'raw_low_two_bits': word & 3, 'presence_only': True, 'collectible_state': 'unknown',
                            'region_inference': 'unknown', 'source': catalog.get('source'),
                            'license': catalog.get('license', 'Not specified')})
    return records


def region_candidates(raw: bytes, stream: dict[str, Any]) -> dict[str, Any]:
    import json
    from pathlib import Path
    if len(raw) != 4194304 or not stream.get('valid'):
        return {'status': 'unsupported', 'entries': [], 'reason': 'Candidate sample-family stream guard did not pass'}
    facts = json.loads((Path(__file__).resolve().parent / 'data' / 'region_counter_candidates.json').read_text(encoding='utf-8'))
    entries = []
    for fact in facts['entries']:
        value = struct.unpack_from('<I', raw, fact['serialized_offset'])[0]
        sensible = value <= fact['source_total']
        entries.append({**fact, 'raw_u32_le': value,
                        'status': 'community_correlated' if sensible else 'candidate',
                        'candidate_count': value if sensible else None,
                        'candidate_remaining_to_source_target': fact['source_total'] - value if sensible else None,
                        'within_source_target': sensible})
    return {'status': 'community_correlated', 'source': facts['source'], 'license': facts['license'],
            'basis': facts['basis'], 'entries': entries,
            'warning': 'Source maxima are category targets from a community autosplitter, not a universal version/DLC contract. No region completion percentage is calculated. ' + WARNING}


def reference_differences(inventory: dict[str, Any], catalog: dict[str, Any]) -> dict[str, Any]:
    import json
    from pathlib import Path
    if inventory.get('status') == 'unsupported':
        return {'status': 'unsupported', 'groups': []}
    facts = json.loads((Path(__file__).resolve().parent / 'data' / 'reference_category_ids.json').read_text(encoding='utf-8'))
    present = {item['id'] for item in inventory['records']}
    groups = []
    for category, fact in facts['groups'].items():
        reference = set(fact['reference_ids'])
        absent = sorted(reference - present)
        observed_category = {item['id'] for item in inventory['records'] if any(name.startswith(category) for name in item['candidate_names'])}
        groups.append({'internal_name_prefix': category, 'reference_count': len(reference),
                       'present_reference_ids': sorted(reference & present), 'absent_reference_count': len(absent),
                       'observed_category_ids_outside_reference': sorted(observed_category - reference),
                       'reference_present_user_absent': [{'id': identifier, 'candidate_names': catalog['mapping'].get(int(identifier,16), []),
                            'collectible_state': 'unknown', 'region_inference': 'unknown'} for identifier in absent]})
    return {'status': facts['status'], 'warning': facts['warning'], 'sources': facts['sources'], 'groups': groups}
