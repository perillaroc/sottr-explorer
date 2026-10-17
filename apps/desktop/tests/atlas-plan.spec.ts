// 收集图鉴计划（SX-022）：前端镜像实现必须与 Rust CLI 黄金输出逐字段一致。
//
// `plan.expected.json` 由 `sottr-cli plan` 对 `analysis.json`（确定性观察集合）生成；
// 任何一侧改动规则都会让本测试失败。
import { describe, expect, it } from "vitest";
import { productionAtlas, buildPlan, findPlanItem, regionDisplayName } from "../src/data/atlas";
import expected from "./fixtures/atlas/plan.expected.json";
import analysis from "./fixtures/atlas/analysis.json";

function observedIds(): string[] {
  const records = analysis.results[0].report.observations.inventory.records as { internal_id: string }[];
  return records.map((record) => record.internal_id);
}

describe("atlas plan parity with CLI", () => {
  it("matches the golden CLI output", () => {
    const plan = buildPlan(productionAtlas, observedIds());
    // CLI 输出额外带 `input_ref`（命令行上下文，不属于计划本体）。
    const { input_ref: _inputRef, ...golden } = expected as Record<string, unknown> & {
      input_ref?: string;
    };
    expect(plan).toEqual(golden);
  });

  it("covers every catalog item exactly once across regions", () => {
    const plan = buildPlan(productionAtlas, []);
    const ids = plan.regions.flatMap((region) => region.groups.flatMap((group) => group.items.map((item) => item.id)));
    expect(ids.length).toBe(productionAtlas.totals.items);
    expect(new Set(ids).size).toBe(productionAtlas.totals.items);
  });

  it("separates special regions from the main totals", () => {
    const plan = buildPlan(productionAtlas, []);
    const main = plan.regions.filter((region) => region.kind === "main");
    const special = plan.regions.filter((region) => region.kind === "special");
    expect(main.length).toBe(9);
    expect(special.map((region) => region.slug).sort()).toEqual(["_challenge_tombs", "_special", "croft_manor"]);
    expect(main.reduce((sum, region) => sum + region.total, 0)).toBe(plan.main_total);
    expect(plan.main_total).toBe(370);
  });

  it("resolves item lookups and display names", () => {
    const plan = buildPlan(productionAtlas, []);
    const paititi = plan.regions.find((region) => region.slug === "paititi");
    expect(paititi).toBeTruthy();
    expect(regionDisplayName(paititi!)).toBe("帕依提提");
    expect(paititi!.total).toBe(159);

    const sample = findPlanItem(plan, "0xF97006F6");
    expect(sample?.item.display_name).toBe("隐秘之城的谜团");
    expect(sample?.region.slug).toBe("_special");
  });

  it("marks settled items as collected and lists unknown ids separately", () => {
    const plan = buildPlan(productionAtlas, ["0xF97006F6", "0xdeadbeef", "0x1234"]);
    expect(findPlanItem(plan, "0xF97006F6")?.item.status).toBe("collected");
    expect(plan.observed_not_in_catalog).toContain("0xDEADBEEF");
    expect(plan.observed_not_in_catalog).toContain("0X1234");
  });

  it("includes unverified relative position and acquisition text for located items", () => {
    const plan = buildPlan(productionAtlas, []);
    const located = plan.regions
      .flatMap((region) => region.groups.flatMap((group) => group.items))
      .find((item) => item.position !== undefined);
    expect(located).toBeTruthy();
    expect(located!.position!.space).toBe("unit_normalized_unverified");
    expect(located!.location_hint).toContain("未校准");
    expect(located!.acquisition_text.length).toBeGreaterThan(0);
  });
});
