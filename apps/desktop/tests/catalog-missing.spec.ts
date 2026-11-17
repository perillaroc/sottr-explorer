// SX-021 阶段 C2a：前端缺失清单与 CLI/Rust 核心实现的一致性（黄金输出对照）。
//
// 夹具由 Rust CLI 生成：fixtures/missing/{catalog.json,analysis.json} 经
// `sottr-cli missing` 产出 expected.json（见 data/catalog/README.md 用法）。
// 任何一边规则变化都必须同时更新夹具，否则此测试失败。
import { describe, expect, it } from "vitest";

import analysisFixture from "./fixtures/missing/analysis.json";
import catalogFixture from "./fixtures/missing/catalog.json";
import expectedFixture from "./fixtures/missing/expected.json";
import {
  catalogScopeNotes,
  computeMissing,
  dlcCollectionProgress,
  productionCatalog,
  REASON_OPTIONAL,
  REASON_UNRESOLVED,
  type Catalog,
} from "../src/data/catalog";
import { optionalReferenceStatus } from "../src/data/optional-references";

describe("computeMissing（与 Rust 核心一致）", () => {
  it("对黄金夹具输出与 CLI missing 完全一致", () => {
    const catalog = catalogFixture as unknown as Catalog;
    const observed = analysisFixture.results
      .flatMap((result) => result.report?.observations.inventory.records ?? [])
      .map((record) => record.internal_id)
      .filter((id): id is string => typeof id === "string");

    const report = computeMissing(catalog, observed);
    const expected = expectedFixture as Record<string, unknown>;
    const expectedReport = Object.fromEntries(
      Object.entries(expected).filter(([key]) => key !== "input_ref"),
    );

    expect(report).toEqual(expectedReport);
  });

  it("真实目录：必收子集、未闭合与可选类别保护成立", () => {
    const report = computeMissing(productionCatalog, ["0xF97006F6", "0xDEADBEEF", "bad-id"]);

    const artifacts = report.categories.find((category) => category.id === "artifacts");
    expect(artifacts?.required_for_100).toBe(true);
    expect(artifacts?.total).toBe(253);
    expect(artifacts?.observed).toBe(1);
    expect(artifacts?.missing).toBe(252);
    expect(artifacts?.items).toHaveLength(252);

    const tombs = report.categories.find((category) => category.id === "challenge_tombs");
    expect(tombs?.required_for_100).toBeNull();
    expect(tombs?.missing).toBeUndefined();
    expect(tombs?.items).toBeUndefined();
    expect(tombs?.reason).toBe(REASON_UNRESOLVED);

    const backpacks = report.categories.find((category) => category.id === "explorer_backpacks");
    expect(backpacks?.reason).toBe(REASON_OPTIONAL);
    expect(backpacks?.items).toBeUndefined();

    expect(report.observed_not_in_catalog).toContain("0xDEADBEEF");
    expect(report.observed_not_in_catalog).toContain("BAD-ID");
    expect(report.observed_items).toBe(1);
  });

  it("DLC 故事条目按 7 个合集分组，不计入必收", () => {
    const groups = dlcCollectionProgress(productionCatalog, []);
    expect(groups.map((group) => group.label)).toEqual([
      "DLC01",
      "DLC02",
      "DLC03",
      "DLC04",
      "DLC05",
      "DLC06",
      "DLC07",
    ]);
    expect(groups.reduce((sum, group) => sum + group.total, 0)).toBe(42);
    expect(groups.every((group) => group.observed === 0)).toBe(true);

    const dlc01 = Object.entries(productionCatalog.items).find(
      ([, item]) => item.is_dlc && item.collection === "DLC01_COLLECT_STORY",
    );
    expect(dlc01).toBeDefined();
    const withOne = dlcCollectionProgress(productionCatalog, [dlc01![0]]);
    expect(withOne.find((group) => group.label === "DLC01")?.observed).toBe(1);
  });

  it("可选类别参考状态：只有保险箱有区域计数器", () => {
    expect(optionalReferenceStatus("strongboxes")).toBe("verified");
    expect(optionalReferenceStatus("explorer_backpacks")).toBe("unknown");
    expect(optionalReferenceStatus("archivist_maps")).toBe("unknown");
    expect(optionalReferenceStatus("crypt_entrances")).toBe("unknown");
    expect(optionalReferenceStatus("gym_training")).toBe("not_applicable");
  });

  it("P2-5 口径：_special 57 条与挑战古墓 12 名 / 8 ID", () => {
    const notes = catalogScopeNotes(productionCatalog);
    expect(notes.specialTotal).toBe(57);
    expect(notes.specialWithUnit).toBe(23);
    expect(notes.specialWithoutUnit).toBe(34);
    expect(notes.challengeTombIds).toBe(8);
    expect(notes.challengeTombNames).toBe(12);
    expect(notes.sharedChallengeId).toBe("0xC9D62DEE");
    expect(notes.sharedChallengeNames).toHaveLength(5);
  });
});
