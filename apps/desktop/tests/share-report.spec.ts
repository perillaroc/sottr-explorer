// 分享报告（P1-5）：摘要模型、自包含 HTML、脱敏与转义。
//
// 只验证报告口径：数字来自图鉴计划与报告结论；不含路径/用户名/哈希/外部资源。
import { describe, expect, it } from "vitest";
import { buildCompleteReport } from "../src/data/mockReports";
import { buildPlan, demoObservedIds, productionAtlas, type CollectionPlan, type PlanItem } from "../src/data/atlas";
import { renderShareReport, shareReportModel } from "../src/app/share-report";

const SEED = { compressedSha256: "ef".repeat(32), compressedBytes: 597_590 };
const REPORT = buildCompleteReport(SEED);
const META = { catalogVersion: productionAtlas.catalog_version, generatedAt: "2026-11-16 12:00:00" };

const emptyPlan = buildPlan(productionAtlas, []);
const observedPlan = buildPlan(productionAtlas, demoObservedIds);

describe("分享报告", () => {
  it("摘要数字与计划一致：主区域、必收、缺失必收", () => {
    const model = shareReportModel(emptyPlan, REPORT);
    expect(model.mainTotal).toBe(370);
    expect(model.mainCollected).toBe(0);
    expect(model.requiredTotal).toBe(352);
    expect(model.requiredCollected).toBe(0);
    expect(model.missingRequired).toBe(352);
    expect(model.overallPercent).toBeCloseTo(66.204_82, 4);
    expect(model.regions.length).toBe(emptyPlan.regions.length);
    expect(model.missing.length).toBeGreaterThan(0);
  });

  it("已观察条目的摘要与计划状态一致", () => {
    const model = shareReportModel(observedPlan, REPORT);
    expect(model.mainCollected).toBe(observedPlan.main_collected);
    expect(model.requiredCollected).toBe(352 - model.missingRequired);
    const missingIds = model.missing.flatMap((group) => group.items.map((item) => item.id));
    expect(missingIds).toHaveLength(model.missingRequired);
    expect(new Set(missingIds).size).toBe(missingIds.length);
  });

  it("HTML 自包含、离线可读：无外部资源、无路径/用户名/哈希", () => {
    const html = renderShareReport(emptyPlan, REPORT, META);
    expect(html.startsWith("<!DOCTYPE html>")).toBe(true);
    expect(html).toContain('<meta charset="utf-8">');
    expect(html).not.toContain("<script");
    expect(html).not.toContain("<link");
    expect(html).not.toContain("<img");
    expect(html).not.toContain("http://");
    expect(html).not.toContain("https://");
    expect(html).not.toContain("save1.dat");
    expect(html).not.toContain("sha256");
    expect(html).toContain("@media print");
    expect(html).toContain("不含存档路径");
    expect(html).toContain("地图缩略图未包含");
    expect(html).toContain(String(productionAtlas.catalog_version));
  });

  it("缺失清单逐条列出：行数与缺失必收数一致", () => {
    const html = renderShareReport(emptyPlan, REPORT, META);
    const model = shareReportModel(emptyPlan, REPORT);
    expect((html.match(/class="id"/g) ?? []).length).toBe(model.missingRequired);
    const first = model.missing[0].items[0];
    expect(html).toContain(first.id);
    expect(html).toContain(first.name);
  });

  it("动态文本被转义；同一输入重复生成结果一致", () => {
    const item = {
      ...emptyPlan.regions[0].groups[0].items[0],
      id: "0x00000001",
      display_name: '<script>alert("x")</script>',
      status: "not_collected",
      required_for_100: true,
    } as PlanItem;
    const plan: CollectionPlan = {
      ...emptyPlan,
      regions: [
        {
          slug: "test",
          region_hash: "test",
          name_zh: "Region <b>",
          name_en: "Region <b>",
          kind: "main",
          collected: 0,
          total: 1,
          groups: [
            {
              category: "artifacts",
              subcategory: "relics",
              name: "Relics <i>",
              required_for_100: true,
              collected: 0,
              total: 1,
              acquisition_rule_key: "artifacts",
              items: [item],
            },
          ],
        },
        ...emptyPlan.regions.slice(1),
      ],
    };
    const html = renderShareReport(plan, REPORT, META);
    expect(html).not.toContain("<script>alert");
    expect(html).toContain("&lt;script&gt;alert");
    expect(html).toContain("Region &lt;b&gt;");
    expect(renderShareReport(plan, REPORT, META)).toBe(html);
  });
});
