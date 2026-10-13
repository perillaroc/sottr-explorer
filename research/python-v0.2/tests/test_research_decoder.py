import json
import os
from pathlib import Path
import struct
import sys
import unittest
import zlib
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import research_decoder as decoder
import sottr_inspect as app


def fixture():
    raw = bytearray(4194304)
    struct.pack_into('<I', raw, 0xBFD0, 4112)
    struct.pack_into('<I', raw, 0xBFD4, (1028 << 8) | 28)
    return raw


class CandidateTests(unittest.TestCase):
    def test_challenge_tomb_not_enumerated(self):
        import tempfile
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'challengetomb.dat').write_bytes(zlib.compress(fixture()))
            (root / 'save1.dat').write_bytes(zlib.compress(fixture()))
            self.assertEqual([p.name for p in app.enumerate_saves([root])], ['save1.dat'])

    def test_packed_valid_and_bounds(self):
        raw = fixture()
        self.assertTrue(decoder.packed_sections(raw)['valid'])
        struct.pack_into('<I', raw, 0xBFD4, 28)
        self.assertFalse(decoder.packed_sections(raw)['valid'])
        struct.pack_into('<I', raw, 0xBFD0, 0xFFFFFFFC)
        self.assertFalse(decoder.packed_sections(raw)['valid'])
        self.assertFalse(decoder.packed_sections(b'x')['valid'])

    def test_partial_prefix_preserves_raw_word_and_skips_zero_id(self):
        raw = fixture()
        struct.pack_into('<II', raw, 0x44, 123, 0xFFFFFFFD)
        struct.pack_into('<II', raw, 0x4C, 0, 0xFFFFFFFC)
        cat = {'mapping': {123: ['A', 'B']}, 'source': 'test', 'license': 'test'}
        result = decoder.prefix_inventory(raw, cat)
        self.assertEqual(result['nonzero_records'], 1)
        self.assertEqual(result['zero_id_slots'], 511)
        item = result['records'][0]
        self.assertEqual(item['raw_following_word'], 0xFFFFFFFD)
        self.assertEqual(item['raw_low_two_bits'], 1)
        self.assertEqual(item['collectible_state'], 'unknown')
        self.assertEqual(item['region_inference'], 'unknown')

    def test_overflow_stride_and_stale_tail_ignored(self):
        raw = fixture()
        start = 0xBFD4 + 16
        for i, word in enumerate([1, 3, 5, 13]):
            struct.pack_into('<II', raw, start + 8 * i, 123+i, word)
        # Bogus stale section beyond declared stream must never be scanned.
        struct.pack_into('<I', raw, 0xBFD4+4112, (1028 << 8) | 28)
        struct.pack_into('<II', raw, 0xBFD4+4112+16, 456, 3)
        stream = decoder.packed_sections(raw)
        result = decoder.overflow_inventory(raw, {'mapping': {}}, stream)
        self.assertEqual(len(result), 4)
        self.assertEqual([r['raw_following_word'] for r in result], [1, 3, 5, 13])

    def test_unknown_evidence_stays_unknown(self):
        raw = fixture()
        struct.pack_into('<f', raw, 0x28, 100)
        self.assertEqual(decoder.overall_candidate(raw)['status'], 'community_correlated')
        self.assertIsNone(app.SaveSlot('save1.dat').overall_completion_percent)
        with self.assertRaises(ValueError):
            app.SaveSlot('save1.dat', overall_completion_percent=100)
        struct.pack_into('<f', raw, 0x28, float('nan'))
        self.assertIsNone(decoder.overall_candidate(raw)['candidate_percent'])
        self.assertEqual(decoder.overall_candidate(b'0'*4194304)['status'], 'unsupported')

    def test_reference_presence_not_truth_state(self):
        result = decoder.reference_differences({'status': 'candidate', 'records': []}, {'mapping': {}})
        self.assertEqual(sum(g['reference_count'] for g in result['groups']), 99)
        self.assertTrue(all(item['collectible_state'] == 'unknown' for g in result['groups'] for item in g['reference_present_user_absent']))
        self.assertEqual(decoder.reference_differences({'status': 'unsupported'}, {})['groups'], [])

    def test_regions_preserve_anomalies(self):
        raw = fixture()
        result = decoder.region_candidates(raw, decoder.packed_sections(raw))
        self.assertEqual(len(result['entries']), 43)
        first = result['entries'][0]
        struct.pack_into('<I', raw, first['serialized_offset'], 0xFFFFFFFF)
        result = decoder.region_candidates(raw, decoder.packed_sections(raw))['entries'][0]
        self.assertIsNone(result['candidate_count'])
        self.assertEqual(result['raw_u32_le'], 0xFFFFFFFF)

    @unittest.skipUnless(os.environ.get('SOTTR_PUBLIC_DIR'), 'Public provenance samples are opt-in')
    def test_public_provenance_samples(self):
        root = Path(os.environ['SOTTR_PUBLIC_DIR'])
        observed = 0
        for path in root.glob('*/save*.dat'):
            raw = zlib.decompress(path.read_bytes())
            stream = decoder.packed_sections(raw)
            self.assertTrue(stream['valid'], str(path))
            self.assertEqual(len(decoder.region_candidates(raw, stream)['entries']), 43)
            self.assertIsNotNone(decoder.overall_candidate(raw)['candidate_percent'])
            observed += 1
        self.assertEqual(observed, 20)
        medium = zlib.decompress((root/'SOTTR-100-percent-medium/save1.dat').read_bytes())
        self.assertEqual(decoder.overall_candidate(medium)['candidate_percent'], 100.0)
        early = zlib.decompress((root/'SOTTR-01-cozumel/save1.dat').read_bytes())
        self.assertEqual(decoder.prefix_inventory(early, {'mapping': {}})['nonzero_records'], 28)
