import hashlib
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
import unittest
import zlib

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import sottr_inspect as app


class DecompressionTests(unittest.TestCase):
    def test_round_trip(self):
        raw = b"hello" * 100
        state, result = app.bounded_decompress(zlib.compress(raw))
        self.assertTrue(state["valid"])
        self.assertTrue(state["eof"])
        self.assertEqual(state["trailing_bytes"], 0)
        self.assertEqual(result, raw)

    def test_corrupt(self):
        state, result = app.bounded_decompress(b"not zlib")
        self.assertIsNone(result)
        self.assertEqual(state["error"], "invalid_zlib_stream")

    def test_checksum_corrupt(self):
        blob = bytearray(zlib.compress(b"x" * 100))
        blob[-1] ^= 1
        state, result = app.bounded_decompress(bytes(blob))
        self.assertIsNone(result)
        self.assertEqual(state["error"], "invalid_zlib_stream")

    def test_truncated(self):
        state, result = app.bounded_decompress(zlib.compress(b"payload")[:-2])
        self.assertIsNone(result)
        self.assertFalse(state["eof"])
        self.assertEqual(state["error"], "truncated_or_incomplete_zlib_stream")

    def test_trailing(self):
        state, result = app.bounded_decompress(zlib.compress(b"payload") + b"tail")
        self.assertEqual(result, b"payload")
        self.assertEqual(state["trailing_bytes"], 4)
        self.assertEqual(state["trailing_sha256"], hashlib.sha256(b"tail").hexdigest())

    def test_concatenated_streams_are_trailing(self):
        state, result = app.bounded_decompress(zlib.compress(b"one") + zlib.compress(b"two"))
        self.assertEqual(result, b"one")
        self.assertGreater(state["trailing_bytes"], 0)

    def test_oversized(self):
        state, result = app.bounded_decompress(zlib.compress(b"x" * 1000000), 1000)
        self.assertIsNone(result)
        self.assertEqual(state["error"], "expanded_size_limit_exceeded")

    def test_exact_limit(self):
        self.assertEqual(app.bounded_decompress(zlib.compress(b"x" * 1000), 1000)[1], b"x" * 1000)

    def test_empty_stream(self):
        self.assertEqual(app.bounded_decompress(zlib.compress(b""))[1], b"")
        self.assertIsNone(app.bounded_decompress(b"")[1])


class ObservationTests(unittest.TestCase):
    def test_diffs(self):
        result = app.compare_bytes(b"\x00\x01\x02\x00", b"\x01\x01\x03\xffX")
        self.assertEqual(result["changed_bytes_including_length_difference"], 4)
        self.assertEqual(result["changed_bits_in_common_range"], 10)
        self.assertEqual(result["contiguous_runs"], [{"start": 0, "end_exclusive": 1, "length": 1}, {"start": 2, "end_exclusive": 5, "length": 3}])

    def test_equal_and_empty(self):
        self.assertEqual(app.compare_bytes(b"same", b"same")["contiguous_run_count"], 0)
        self.assertEqual(app.compare_bytes(b"", b"")["changed_bits_in_common_range"], 0)
        self.assertEqual(app.compare_bytes(b"", b"xy")["length_difference_bytes"], 2)

    def test_limits_and_pages(self):
        result = app.compare_bytes(b"\0" * 8192, b"\1\0" * 4096, max_runs=2, max_candidates=1)
        self.assertTrue(result["runs_truncated"])
        self.assertEqual(len(result["numeric_candidates"]), 1)
        self.assertEqual(len(result["changed_bytes_per_page"]), 2)

    def test_ascii_offsets(self):
        result = app.ascii_strings(b"\0hello\0world\0", 1)
        self.assertEqual(result["listed"][0]["offset"], 1)
        self.assertTrue(result["list_truncated"])

    def test_nonfinite_numbers_json_safe(self):
        result = app.numeric_candidates(struct.pack("<d", float("inf")), 0)
        json.dumps(result, allow_nan=False)
        self.assertIsNone(result["f64_le"])

    def test_models_reject_unverified_progress(self):
        with self.assertRaises(ValueError):
            app.SaveSlot("save1.dat", overall_completion_percent=0)
        with self.assertRaises(ValueError):
            app.CollectibleItem("id", state=app.CollectibleState.COLLECTED)
        with self.assertRaises(ValueError):
            app.CollectibleCategory("id", collected=1, total=0, evidence=app.Evidence("validated", "test", ["fixture"]))
        self.assertIsNone(app.SaveSlot("save202.dat").role)
        self.assertIsNone(app.SaveSlot("save202.dat").overall_completion_percent)

    def test_catalog_validation(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "catalog.json"
            invalid = ['[]', '{"source":NaN,"mapping":{}}', '{"source":Infinity,"mapping":{}}',
                       '{"source":{},"mapping":{}}', '{"mapping":{"0x1":["A"],"0x1":["B"]}}']
            for document in invalid:
                with self.subTest(document=document):
                    path.write_text(document)
                    with self.assertRaises(ValueError):
                        app.load_catalog(path)
            path.write_text('{"mapping":{"0x1":["A"],"1":["B"]}}')
            self.assertEqual(app.load_catalog(path)["mapping"][1], ["A", "B"])

    def test_duplicate_names_preserved(self):
        catalog = {"source": "test", "sha256": "test", "mapping": {0x12345678: ["A", "B", "A"]}}
        result = app.scan_catalog(struct.pack("<II", 0x12345678, 3), catalog)
        self.assertEqual(result["total_occurrences"]["little"], 1)
        self.assertEqual(result["occurrences"][0]["candidate_names"], ["A", "B", "A"])
        self.assertEqual(result["occurrences"][0]["collectible_state"], "unknown")


class FileSafetyTests(unittest.TestCase):
    def test_end_to_end_hash_unchanged_and_nulls(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            source = base / "inputs"
            source.mkdir()
            blob = zlib.compress(b"example data\0" * 100)
            (source / "save1.dat").write_bytes(blob)
            (source / "save202.dat").write_bytes(blob)
            (source / "profile.dat").write_bytes(zlib.compress(b"profile"))
            before = {p: p.read_bytes() for p in source.iterdir()}
            report = app.run_analysis([source], base / "report")
            self.assertEqual({p: p.read_bytes() for p in source.iterdir()}, before)
            self.assertTrue(report["integrity"]["all_input_hashes_unchanged"])
            self.assertIsNone(report["unsupported"]["item_progress"])
            self.assertEqual(len(report["known_observations"]["duplicate_compressed_groups"]), 1)
            self.assertEqual(report["known_observations"]["pairs_analyzed"], 1)
            self.assertTrue((base / "report" / "research.log").is_file())

    def test_output_overlap_and_existing_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            source = base / "inputs"
            source.mkdir()
            for output in (base, source, source / "report"):
                with self.assertRaises(ValueError):
                    app.validate_output(output, [source])
            output = base / "existing"
            output.mkdir()
            with self.assertRaises(ValueError):
                app.validate_output(output, [source])

    def test_symlink_input_and_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            source = base / "inputs"
            source.mkdir()
            target = base / "real.dat"
            target.write_bytes(b"test")
            try:
                (source / "save1.dat").symlink_to(target)
                (base / "alias").symlink_to(source, target_is_directory=True)
            except OSError:
                self.skipTest("Symlink creation unavailable")
            self.assertEqual(app.enumerate_saves([source]), [])
            with self.assertRaises(ValueError):
                app.validate_output(base / "alias" / "report", [source])

    @unittest.skipUnless(os.environ.get("SOTTR_SAMPLE_DIR"), "Private real fixtures are opt-in via SOTTR_SAMPLE_DIR")
    def test_private_real_fixtures(self):
        root = Path(os.environ["SOTTR_SAMPLE_DIR"]).resolve()
        with tempfile.TemporaryDirectory() as temporary:
            report = app.run_analysis([root], Path(temporary) / "report")
        inventory = {r["name"]: r for r in report["known_observations"]["inventory"]}
        self.assertEqual(inventory["profile.dat"]["uncompressed_bytes"], 25600)
        for name in ("save1.dat", "save202.dat", "save204.dat", "save206.dat"):
            self.assertEqual(inventory[name]["uncompressed_bytes"], 4194304)
            self.assertTrue(inventory[name]["decompression"]["eof"])
            self.assertEqual(inventory[name]["decompression"]["trailing_bytes"], 0)
        self.assertEqual(inventory["save204.dat"]["compressed_sha256"], inventory["save206.dat"]["compressed_sha256"])
        pairs = report["known_observations"]["pairwise_comparisons"]
        pair = next(p for p in pairs if {Path(p["file_a"]).name, Path(p["file_b"]).name} == {"save1.dat", "save202.dat"})
        self.assertEqual(pair["changed_bytes_including_length_difference"], 9)


if __name__ == "__main__":
    unittest.main()
