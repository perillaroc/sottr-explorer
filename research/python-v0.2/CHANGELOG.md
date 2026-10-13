# v0.2.0 — independent read-only research extension

## Stronger source-backed observations

- The float32 at expanded offset 0x28 agrees with 100.0 in independently authored public 100%-labeled saves. Earlier Cozumel checkpoints have smaller increasing values. A 92%-complete region does not mean 92% overall. Archive-level 100% labels cannot label every enclosed backup; one NG+ backup reads 92.375221.
- Forty-three counters from Atorizil's community live-memory autosplitter align with 104-byte serialized rows, whereas live region records use a 272-byte stride. This crosswalk is empirical: the ASL itself never documents save offsets. Values are labeled community_correlated and separated from validated models.
- Original samples show Paititi's cache candidate counter changing 31 → 32, matching the new overflow ID `0xDD26A1BC`, internal name `CollectibleTypeSurvivalCache_CT_Serpent_Hollow_01`. The latest original save has 512 prefix plus 5 nonzero overflow records, versus 512 plus 4 in the earlier pair.
- Public early saves grow from 28 to 44 nonzero prefix IDs. The ordered map-before-artifact checkpoints support an acquisition/interacted history rather than a static catalog or all revealed items. This does not prove ownership flags or exact UI labels.
- Old public companion DWORDs may contain large values. Nonzero ID observations have low two bits 1/3, but enum semantics remain unknown. Zero-ID slots may have arbitrary companion values and any low bits.
- Independently implemented the empirical packed stream grammar, with exact declared-end checks and bounded iteration. Type28 is an optional 512 × 8-byte overflow table after a 16-byte section header, not 256 × 16-byte hash slots. No assumption about companion values is used for structure validation.
- The old mirrored prefix can lag recent acquisitions. The current prefix and active declared-stream overflow are used; mirror disagreement is not a rejection rule, and stale bytes after the declared stream are excluded.

## Practical deliverables

- Explicit `present_item_ids.csv/json`, preserving both candidate names for duplicate IDs and unknown IDs, raw companion DWORD, low two bits, candidate table source and exact offsets
- Separate `derived_candidates` for overall percent and regional category counts, retaining source targets and anomalies; no region percentages or authoritative missing checklist
- `candidate_packed_stream` provides all walked sections for reproducibility
- All original model guards preserved: unsupported truth fields stay null/unknown, and candidate evidence cannot create known progress
- A separately labeled reference-only difference list compares four bounded internal-name groups shared by two author-labeled 100% saves; all returned item states remain unknown. Original current saves differ by 10 cache IDs, 1 monolith ID and 1 crypt-treasure ID. This is not a universal or all-DLC checklist.
- Raw duplicate records are retained and unique IDs counted separately.
- Schema version 2, tool version 0.2.0, original CLI retained; no input writes or network access

## Verified scope

Twenty-nine tests passed with real fixtures enabled: original 5 files (4 ordinary saves plus profile), 20 public ordinary saves, and synthetic malformed/edge structures. Separate full command runs record input hashes before/after. Public `challengetomb.dat` is deliberately outside CLI discovery.

## Still unverified

Exact companion bit semantics, region completion formula and DLC/version universality, automatic/manual/backup slot roles, localized display names, and a complete actionable missing-item list. This release does not infer them. Public archive bytes and unlicensed source code are not bundled.
