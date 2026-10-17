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
  computeMissing,
  productionCatalog,
  REASON_OPTIONAL,
  REASON_UNRESOLVED,
  type Catalog,
} from "../src/data/catalog";

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
});
