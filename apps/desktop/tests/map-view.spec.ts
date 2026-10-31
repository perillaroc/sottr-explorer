// 地图视图派生（2026-10-31 地图查询与详情增强·阶段二）：
// 底图目录排序与计数、默认底图、条目/分组/筛选、无坐标/无底图统计、全量覆盖统计。
import { describe, expect, it } from "vitest";
import {
  defaultMapBasemap,
  filterMapEntries,
  mapCoverage,
  mapEntriesForBasemap,
  mapGroups,
  mapOptions,
  mapOptionsForRegion,
  regionHasBasemap,
  unmappedInRegion,
  unplacedInBasemap,
} from "../src/app/map-view";
import { buildPlan, demoObservedIds, productionAtlas, type CollectionPlan, type PlanGroup, type PlanItem, type PlanRegion } from "../src/data/atlas";
import { basemapForUnit, basemapsForRegion } from "../src/data/basemaps";
const plan = buildPlan(productionAtlas, demoObservedIds);
const atlas = productionAtlas;

function item(
  id: string,
  unit: number | undefined,
  position?: { x: number; y: number },
  collected = false,
): PlanItem {
  return {
    id,
    display_name: `名称 ${id}`,
    name_en: `Name ${id}`,
    name_zh: `名称 ${id}`,
    internal_name: `internal-${id}`,
    status: collected ? "collected" : "not_collected",
    is_dlc: false,
    required_for_100: true,
    unit: unit
      ? { unit, zone: `zone_${unit}`, display_zh: `子关卡 ${unit}`, display_en: `Sub-level ${unit}`, name_source: "curated" }
      : undefined,
    position: position ? { x: position.x, y: position.y, space: "unit_normalized_unverified" } : undefined,
    acquisition_text: "",
    acquisition_source: "category_rule",
    location_hint: "",
  };
}

function group(category: string, items: PlanItem[], subcategory?: string): PlanGroup {
  return {
    category,
    subcategory,
    name: category,
    required_for_100: true,
    collected: items.filter((entry) => entry.status === "collected").length,
    total: items.length,
    acquisition_rule_key: "",
    items,
  };
}

function region(slug: string, groups: PlanGroup[]): PlanRegion {
  return {
    slug,
    region_hash: `hash-${slug}`,
    kind: "main",
    collected: groups.reduce((sum, entry) => sum + entry.collected, 0),
    total: groups.reduce((sum, entry) => sum + entry.total, 0),
    groups,
  };
}

function syntheticPlan(regions: PlanRegion[]): CollectionPlan {
  return {
    catalog_version: "test",
    main_collected: 0,
    main_total: 0,
    regions,
    observed_not_in_catalog: [],
  };
}

describe("底图目录（真实数据集）", () => {
  it("排除计划里不存在的区域（unnamed），区域/子关卡排序稳定", () => {
    const options = mapOptions(plan, atlas);
    expect(options.length).toBe(39);
    // 区域顺序与计划一致（只取有底图的区域）。
    const regionSlugs = [...new Set(options.map((option) => option.regionSlug))];
    const expected = plan.regions
      .filter((entry) => basemapsForRegion(entry.slug).length > 0)
      .map((entry) => entry.slug);
    expect(regionSlugs).toEqual(expected);
    // 同区域内按最小 unit 升序。
    const paititi = mapOptionsForRegion(plan, atlas, "paititi");
    const units = paititi.map((option) => Math.min(...option.unitIds));
    expect([...units]).toEqual([...units].sort((left, right) => left - right));
  });

  it("条目计数只含有坐标条目；无目录条目的底图仍保留", () => {
    const options = mapOptions(plan, atlas);
    const dense = options.find((option) => option.unitIds.includes(243));
    expect(dense?.regionSlug).toBe("paititi");
    expect(dense?.itemCount).toBe(84);
    expect((dense?.collected ?? 0) + (dense?.uncollected ?? 0)).toBe(84);

    const empty = options.find((option) => option.basemapId === "bm:cozumel/54");
    expect(empty).toBeDefined();
    expect(empty?.itemCount).toBe(0);
  });

  it("默认底图优先有带坐标条目的图；未知区域返回 null", () => {
    const fallback = defaultMapBasemap(plan, atlas, "paititi");
    expect(fallback?.itemCount).toBeGreaterThan(0);
    expect(defaultMapBasemap(plan, atlas, "no_such_region")).toBeNull();
  });

  it("无底图特殊区域：regionHasBasemap 为 false，绝大多数条目计入无底图", () => {
    expect(regionHasBasemap(plan, "_special")).toBe(false);
    const special = plan.regions.find((entry) => entry.slug === "_special")!;
    // 57 项中 2 项位于同时属于主区域的挑战古墓 unit，其余 55 项无底图。
    expect(special.total).toBe(57);
    expect(unmappedInRegion(plan, "_special")).toBe(55);
  });

  it("全量覆盖统计：42 张底图 · 24/65 个子关卡 · 291/470 条可上图", () => {
    const coverage = mapCoverage(plan, atlas);
    expect(coverage).toEqual({
      basemaps: 42,
      units: 65,
      unitsWithBasemap: 24,
      mappable: 291,
      items: 470,
    });
  });
});

describe("地图条目与分组", () => {
  it("条目保持计划顺序，只含该图 unit 且有坐标的条目", () => {
    const basemap = basemapsForRegion("paititi").find((entry) => entry.unit_ids.includes(243));
    expect(basemap).toBeDefined();
    const entries = mapEntriesForBasemap(plan, basemap!);
    expect(entries.length).toBe(84);
    expect(entries.every((entry) => entry.item.unit?.unit === 243)).toBe(true);
    expect(entries.every((entry) => entry.item.position !== undefined)).toBe(true);

    // 与计划里的顺序一致（同 unit 条目；挑战古墓等特殊区域也可能共享同一子关卡）。
    const planned = plan.regions
      .flatMap((region) => region.groups)
      .flatMap((group) => group.items)
      .filter((entry) => entry.unit?.unit === 243 && entry.position)
      .map((entry) => entry.id);
    expect(entries.map((entry) => entry.item.id)).toEqual(planned);
  });

  it("分组保持计划顺序，组计数是整组真实计数、筛选只影响显示条目", () => {
    const basemap = basemapsForRegion("paititi").find((entry) => entry.unit_ids.includes(243))!;
    const entries = mapEntriesForBasemap(plan, basemap);
    const allGroups = mapGroups(entries, { status: "all", category: null, search: "" });
    expect(allGroups.reduce((sum, entry) => sum + entry.total, 0)).toBe(84);
    expect(allGroups.reduce((sum, entry) => sum + entry.items.length, 0)).toBe(84);

    const missingOnly = mapGroups(entries, { status: "missing", category: null, search: "" });
    expect(missingOnly.every((entry) => entry.items.every((item) => item.item.status === "not_collected"))).toBe(true);
    // 组的总数不随筛选变化。
    for (const group of missingOnly) {
      const original = allGroups.find((entry) => entry.category === group.category && entry.subcategory === group.subcategory);
      expect(group.total).toBe(original?.total);
    }
  });

  it("类别与搜索筛选", () => {
    const basemap = basemapsForRegion("paititi").find((entry) => entry.unit_ids.includes(243))!;
    const entries = mapEntriesForBasemap(plan, basemap);
    const category = entries[0].group.category;
    const filtered = filterMapEntries(entries, { status: "all", category, search: "" });
    expect(filtered.length).toBeGreaterThan(0);
    expect(filtered.every((entry) => entry.group.category === category)).toBe(true);

    const target = entries[0].item;
    const searched = filterMapEntries(entries, {
      status: "all",
      category: null,
      search: target.internal_name ?? target.id,
    });
    expect(searched.map((entry) => entry.item.id)).toContain(target.id);
  });
});

describe("无坐标与无底图统计（合成计划）", () => {
  // 243 是真实有底图的 unit（Paititi）；424242 没有底图。
  const withBasemapUnit = 243;
  const withoutBasemapUnit = 424242;
  const testRegion = region("test_region", [
    group("survival_caches", [
      item("a", withBasemapUnit, { x: 0.1, y: 0.1 }, true),
      item("b", withBasemapUnit, { x: 0.2, y: 0.2 }),
      item("c", withBasemapUnit),
      item("d", withoutBasemapUnit, { x: 0.3, y: 0.3 }),
      item("e", undefined, { x: 0.4, y: 0.4 }),
    ]),
  ]);
  const testPlan = syntheticPlan([testRegion]);
  const basemap = basemapForUnit(withBasemapUnit)!;

  it("本图无坐标条目只统计该 unit 内没有 position 的条目", () => {
    expect(unplacedInBasemap(testPlan, basemap)).toBe(1);
  });

  it("区域无底图条目包含无 unit 与无底图 unit 的条目", () => {
    // 424242 没有底图；e 没有 unit。
    expect(unmappedInRegion(testPlan, "test_region")).toBe(2);
    expect(unmappedInRegion(testPlan, "missing_region")).toBe(0);
  });

  it("底图条目只取有坐标的同 unit 条目", () => {
    const entries = mapEntriesForBasemap(testPlan, basemap);
    expect(entries.map((entry) => entry.item.id)).toEqual(["a", "b"]);
  });
});
