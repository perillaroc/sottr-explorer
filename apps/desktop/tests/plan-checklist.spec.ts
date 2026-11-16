// 收集计划清单（P1-2）：分组/筛选/计数与导出渲染的纯函数测试。
//
// 状态来自 `buildPlan`（与 CLI `plan` 黄金一致）；这里只验证“清单视图”的
// 口径：只列未收集、区域 → 子关卡 → 类别、必收/全部、导出转义。
import { describe, expect, it } from "vitest";
import { buildPlan, demoObservedIds, productionAtlas } from "../src/data/atlas";
import { basemapForUnit } from "../src/data/basemaps";
import { countPlan } from "../src/app/collection";
import {
  buildChecklist,
  checklistCategories,
  checklistRegions,
  DEFAULT_PLAN_FILTERS,
  escapeHtml,
  planCsvCell,
  PLAN_CSV_FILE,
  PLAN_HTML_FILE,
  renderPlanCsv,
  renderPlanHtml,
  type ChecklistItem,
  type PlanChecklist,
  type PlanExportMeta,
} from "../src/app/plan-checklist";

const emptyPlan = buildPlan(productionAtlas, []);
const observedPlan = buildPlan(productionAtlas, demoObservedIds);

const meta: PlanExportMeta = {
  catalogVersion: emptyPlan.catalog_version,
  generatedAt: "2026-11-16 12:00:00",
  snapshotTime: "2026-11-16 11:00:00",
  inputRef: "save1.dat",
  share: false,
  scopeText: "范围：仅必收 · 全部区域 · 全部类别",
};

function requiredUncollected(plan: ReturnType<typeof buildPlan>): number {
  let count = 0;
  for (const region of plan.regions) {
    for (const group of region.groups) {
      for (const item of group.items) {
        if (item.required_for_100 === true && item.status === "not_collected") {
          count += 1;
        }
      }
    }
  }
  return count;
}

describe("收集计划清单", () => {
  it("默认口径（仅必收）只列未收集的必收条目，计数与计划一致", () => {
    const checklist = buildChecklist(emptyPlan, DEFAULT_PLAN_FILTERS);
    expect(checklist.items).toBe(352);
    expect(checklist.required).toBe(352);
    expect(checklist.items).toBe(requiredUncollected(emptyPlan));
    expect(checklist.regionCount).toBe(checklist.regions.length);
    // 区域顺序沿用计划（主区域在前、特殊区域在后）。
    const kinds = checklist.regions.map((region) => region.kind);
    const firstSpecial = kinds.indexOf("special");
    if (firstSpecial >= 0) {
      expect(kinds.slice(firstSpecial).every((kind) => kind === "special")).toBe(true);
    }
  });

  it("全部未收集口径包含可选与未闭合条目", () => {
    const all = buildChecklist(emptyPlan, { ...DEFAULT_PLAN_FILTERS, scope: "all" });
    expect(all.items).toBe(countPlan(emptyPlan).uncollected);
    expect(all.items).toBeGreaterThan(352);
    expect(all.required).toBe(352);
    const unresolved = all.regions
      .flatMap((region) => region.units.flatMap((unit) => unit.groups.flatMap((group) => group.items)))
      .filter((item) => item.required === null);
    expect(unresolved.length).toBeGreaterThan(0);
  });

  it("已收集条目从清单消失：计数等于必收总数减去已收集必收", () => {
    const checklist = buildChecklist(observedPlan, DEFAULT_PLAN_FILTERS);
    const collectedRequired = 352 - requiredUncollected(observedPlan);
    expect(collectedRequired).toBeGreaterThan(0);
    expect(checklist.items).toBe(352 - collectedRequired);
  });

  it("区域筛选只保留该区域，计数与区域分组一致", () => {
    const region = emptyPlan.regions.find((entry) => entry.slug === "mission_of_san_juan");
    expect(region).toBeTruthy();
    const expected = region!.groups
      .flatMap((group) => group.items)
      .filter((item) => item.status === "not_collected" && item.required_for_100 === true).length;
    const checklist = buildChecklist(emptyPlan, {
      ...DEFAULT_PLAN_FILTERS,
      region: "mission_of_san_juan",
    });
    expect(checklist.regions).toHaveLength(1);
    expect(checklist.regions[0].slug).toBe("mission_of_san_juan");
    expect(checklist.items).toBe(expected);
    expect(expected).toBeGreaterThan(0);
  });

  it("类别筛选与类别选项计数一致", () => {
    const categories = checklistCategories(emptyPlan, DEFAULT_PLAN_FILTERS);
    const monoliths = categories.find((entry) => entry.id === "monoliths");
    expect(monoliths?.count).toBeGreaterThan(0);
    const checklist = buildChecklist(emptyPlan, { ...DEFAULT_PLAN_FILTERS, category: "monoliths" });
    expect(checklist.items).toBe(monoliths!.count);
    const groups = checklist.regions.flatMap((region) => region.units.flatMap((unit) => unit.groups));
    expect(groups.every((group) => group.category === "monoliths")).toBe(true);
  });

  it("区域选项与清单计数一致（未收集数为 0 的区域不出现）", () => {
    const options = checklistRegions(observedPlan, DEFAULT_PLAN_FILTERS);
    const checklist = buildChecklist(observedPlan, DEFAULT_PLAN_FILTERS);
    for (const region of checklist.regions) {
      expect(options.find((option) => option.slug === region.slug)?.count).toBe(region.remaining);
    }
    expect(options.length).toBe(observedPlan.regions.length);
  });

  it("子关卡可按未收集数降序排序", () => {
    const checklist = buildChecklist(emptyPlan, { ...DEFAULT_PLAN_FILTERS, sort: "remaining" });
    for (const region of checklist.regions) {
      const counts = region.units.map((unit) => unit.remaining);
      expect(counts).toEqual([...counts].sort((left, right) => right - left));
    }
  });

  it("底图与文字下落标记与数据源一致", () => {
    const checklist = buildChecklist(emptyPlan, { ...DEFAULT_PLAN_FILTERS, scope: "all" });
    const items = checklist.regions.flatMap((region) =>
      region.units.flatMap((unit) => unit.groups.flatMap((group) => group.items)),
    );
    for (const item of items) {
      const unit = item.item.unit?.unit ?? null;
      expect(item.hasBasemap).toBe(basemapForUnit(unit) !== null);
      expect(item.hasText).toBe(item.acquisitionSource !== "category_rule");
    }
    expect(items.some((item) => item.hasBasemap)).toBe(true);
    expect(items.some((item) => item.hasText)).toBe(true);
  });

  it("CSV：BOM、表头与行数，危险前缀与引号被转义", () => {
    const checklist = buildChecklist(emptyPlan, DEFAULT_PLAN_FILTERS);
    const csv = renderPlanCsv(checklist);
    expect(csv.startsWith("﻿")).toBe(true);
    const lines = csv.replace(/^﻿/, "").trimEnd().split("\r\n");
    expect(lines).toHaveLength(checklist.items + 1);
    expect(lines[0]).toContain("区域");
    expect(lines[0]).toContain("已收取");
    // 中间行包含真实条目 ID。
    const firstItem = checklist.regions[0].units[0].groups[0].items[0];
    expect(csv).toContain(firstItem.id);

    expect(planCsvCell("=1+1")).toBe("'=1+1");
    expect(planCsvCell("  =前导空白")).toBe("'  =前导空白");
    expect(planCsvCell("a,b")).toBe('"a,b"');
    expect(planCsvCell('a"b')).toBe('"a""b"');
    expect(planCsvCell("a\nb")).toBe('"a\nb"');
    expect(planCsvCell("0x0F28B6EC")).toBe("0x0F28B6EC");
  });

  it("HTML：自包含、可打印、每条一个勾选框，分享模式不含输入文件名", () => {
    const checklist = buildChecklist(emptyPlan, { ...DEFAULT_PLAN_FILTERS, region: "mission_of_san_juan" });
    const html = renderPlanHtml(checklist, meta);
    expect(html.startsWith("<!DOCTYPE html>")).toBe(true);
    expect(html).toContain('<meta charset="utf-8">');
    expect(html).not.toContain("http://");
    expect(html).not.toContain("https://");
    expect(html).toContain("@media print");
    expect((html.match(/☐/g) ?? []).length).toBe(checklist.items);
    expect(html).toContain("save1.dat");
    expect(html).toContain(escapeHtml(meta.scopeText));

    const shared = renderPlanHtml(checklist, { ...meta, share: true, inputRef: null, snapshotTime: null });
    expect(shared).not.toContain("save1.dat");
    expect(shared).toContain("分享模式");
  });

  it("HTML 转义动态文本（名称/线索不注入标签）", () => {
    const item: ChecklistItem = {
      id: "0x00000001",
      name: '<script>alert("x")</script>',
      category: "artifacts",
      categoryLabel: "故事收藏 <b>",
      subcategory: null,
      required: true,
      isDlc: false,
      hasBasemap: true,
      hasText: true,
      locationHint: "A & B",
      acquisitionText: "<img src=x onerror=alert(1)>",
      acquisitionSource: "item_note",
      item: emptyPlan.regions[0].groups[0].items[0],
    };
    const checklist: PlanChecklist = {
      regions: [
        {
          slug: "test",
          label: "Test <Region>",
          kind: "main",
          remaining: 1,
          units: [{ unit: 1, label: "Unit", remaining: 1, groups: [{ category: "artifacts", subcategory: null, label: "G", items: [item] }] }],
        },
      ],
      items: 1,
      required: 1,
      regionCount: 1,
      filters: DEFAULT_PLAN_FILTERS,
    };
    const html = renderPlanHtml(checklist, meta);
    expect(html).not.toContain("<script>alert");
    expect(html).not.toContain("<img src=x");
    expect(html).toContain("&lt;script&gt;");
    expect(html).toContain("A &amp; B");
    expect(escapeHtml('<a href="x">')).toBe("&lt;a href=&quot;x&quot;&gt;");
  });

  it("导出文件名固定为 plan.html / plan.csv", () => {
    expect(PLAN_HTML_FILE).toBe("plan.html");
    expect(PLAN_CSV_FILE).toBe("plan.csv");
  });
});
