// 收集计划（P1-2）：把 `buildPlan` 的区域计划整理成“这次去哪里收”的清单。
//
// 职责边界：
// - 只做筛选、分组、排序与导出渲染，不重新解释收集状态：状态一律来自
//   `buildPlan`（与 Rust 核心 `sottr_core::atlas` 黄金一致）；
// - 清单只列**未收集**条目：已收集的条目属于“已完成”，不进入这次要去的地方；
// - 分组层级固定为 区域 → 子关卡 → 类别，与任务文档一致；区域顺序沿用计划
//   （主区域在前、特殊区域在后），子关卡默认按关卡顺序，可切换为未收集数降序；
// - 导出内容由本模块纯函数生成，后端只负责写入新目录（固定文件名 + 脱敏自检）。

import { regionDisplayName, type CollectionPlan, type PlanItem, type PlanRegion } from "../data/atlas";
import { basemapForUnit } from "../data/basemaps";
import { escapeHtml } from "./html";
import { localizedName, t } from "./i18n";
import { categoryLabel, subcategoryLabel } from "./labels";

export { escapeHtml } from "./html";

/** 清单口径：仅必收条目 / 全部未收集条目（含可选与未闭合）。 */
export type PlanScope = "required" | "all";

/** 子关卡排序：关卡顺序 / 未收集数降序。 */
export type PlanSort = "unit" | "remaining";

export interface PlanFilters {
  scope: PlanScope;
  /** 区域 slug，或 `"all"`。 */
  region: string;
  /** 类别 id，或 `null` 表示全部类别。 */
  category: string | null;
  sort: PlanSort;
}

export const DEFAULT_PLAN_FILTERS: PlanFilters = {
  scope: "required",
  region: "all",
  category: null,
  sort: "unit",
};

export interface ChecklistItem {
  id: string;
  name: string;
  category: string;
  categoryLabel: string;
  subcategory: string | null;
  required: boolean | null;
  isDlc: boolean;
  /** 所属子关卡有底图（地图查询里能打开具体位置）。 */
  hasBasemap: boolean;
  /** 有合集 / 子关卡 / 逐项文字下落（不只是类别通用规则）。 */
  hasText: boolean;
  locationHint: string;
  acquisitionText: string;
  acquisitionSource: PlanItem["acquisition_source"];
  /** 原始计划条目（详情、攻略直链使用）。 */
  item: PlanItem;
}

export interface ChecklistGroup {
  category: string;
  subcategory: string | null;
  label: string;
  items: ChecklistItem[];
}

export interface ChecklistUnit {
  /** 子关卡 ID；`null` 表示区域级（图鉴里没有子关卡的行）。 */
  unit: number | null;
  label: string;
  remaining: number;
  groups: ChecklistGroup[];
}

export interface ChecklistRegion {
  slug: string;
  label: string;
  kind: PlanRegion["kind"];
  /** 当前筛选下该区域未收集数。 */
  remaining: number;
  units: ChecklistUnit[];
}

export interface PlanChecklist {
  regions: ChecklistRegion[];
  /** 当前筛选下的未收集条目数。 */
  items: number;
  /** 其中必收条目数。 */
  required: number;
  /** 涉及的区域数。 */
  regionCount: number;
  filters: PlanFilters;
}

function unitLabel(item: PlanItem): string {
  const unit = item.unit;
  if (!unit) {
    return t("未指定子关卡", "No sub-level");
  }
  return localizedName(unit.display_zh, unit.display_en, unit.zone ?? `#${unit.unit}`);
}

function requiredLabel(required: boolean | null): string {
  if (required === true) {
    return t("必收", "Required");
  }
  if (required === false) {
    return t("可选", "Optional");
  }
  return t("未闭合", "Unresolved");
}

function matchesScope(required: boolean | null, scope: PlanScope): boolean {
  return scope === "all" || required === true;
}

function buildItem(item: PlanItem, groupCategory: string): ChecklistItem {
  return {
    id: item.id,
    name: item.display_name,
    category: groupCategory,
    categoryLabel: categoryLabel(groupCategory, null),
    subcategory: null,
    required: item.required_for_100,
    isDlc: item.is_dlc,
    hasBasemap: basemapForUnit(item.unit?.unit) !== null,
    hasText: item.acquisition_source !== "category_rule",
    locationHint: item.location_hint,
    acquisitionText: item.acquisition_text,
    acquisitionSource: item.acquisition_source,
    item,
  };
}

/** 计算当前筛选下的收集清单。 */
export function buildChecklist(plan: CollectionPlan, filters: PlanFilters = DEFAULT_PLAN_FILTERS): PlanChecklist {
  const regions: ChecklistRegion[] = [];
  let items = 0;
  let required = 0;

  for (const region of plan.regions) {
    if (filters.region !== "all" && region.slug !== filters.region) {
      continue;
    }
    const units = new Map<string, ChecklistUnit>();
    let regionRemaining = 0;

    for (const group of region.groups) {
      if (filters.category !== null && group.category !== filters.category) {
        continue;
      }
      const groupName = group.subcategory ? (subcategoryLabel(group.subcategory) ?? group.name) : categoryLabel(group.category, group.name);
      for (const planItem of group.items) {
        if (planItem.status !== "not_collected") {
          continue;
        }
        if (!matchesScope(planItem.required_for_100, filters.scope)) {
          continue;
        }
        const checklistItem = buildItem(planItem, group.category);
        checklistItem.subcategory = group.subcategory ?? null;
        checklistItem.categoryLabel = groupName;
        const key = planItem.unit ? String(planItem.unit.unit) : "none";
        let unit = units.get(key);
        if (!unit) {
          unit = {
            unit: planItem.unit?.unit ?? null,
            label: unitLabel(planItem),
            remaining: 0,
            groups: [],
          };
          units.set(key, unit);
        }
        let checklistGroup = unit.groups.find(
          (entry) => entry.category === group.category && entry.subcategory === (group.subcategory ?? null),
        );
        if (!checklistGroup) {
          checklistGroup = {
            category: group.category,
            subcategory: group.subcategory ?? null,
            label: groupName,
            items: [],
          };
          unit.groups.push(checklistGroup);
        }
        checklistGroup.items.push(checklistItem);
        unit.remaining += 1;
        regionRemaining += 1;
        items += 1;
        if (planItem.required_for_100 === true) {
          required += 1;
        }
      }
    }

    if (regionRemaining === 0) {
      continue;
    }

    const orderedUnits = [...units.values()];
    if (filters.sort === "remaining") {
      orderedUnits.sort((left, right) => {
        const diff = right.remaining - left.remaining;
        if (diff !== 0) {
          return diff;
        }
        return unitOrder(left.unit) - unitOrder(right.unit);
      });
    } else {
      orderedUnits.sort((left, right) => unitOrder(left.unit) - unitOrder(right.unit));
    }

    regions.push({
      slug: region.slug,
      label: regionDisplayName(region),
      kind: region.kind,
      remaining: regionRemaining,
      units: orderedUnits,
    });
  }

  return { regions, items, required, regionCount: regions.length, filters };
}

function unitOrder(unit: number | null): number {
  return unit === null ? Number.MAX_SAFE_INTEGER : unit;
}

/** 当前筛选的类别选项（计划里未收集的类别与计数）。 */
export function checklistCategories(
  plan: CollectionPlan,
  filters: PlanFilters,
): { id: string; label: string; count: number }[] {
  const scoped = buildChecklist(plan, { ...filters, region: filters.region, category: null });
  const seen = new Map<string, { id: string; label: string; count: number }>();
  for (const region of scoped.regions) {
    for (const unit of region.units) {
      for (const group of unit.groups) {
        const entry = seen.get(group.category) ?? {
          id: group.category,
          label: categoryLabel(group.category, group.label),
          count: 0,
        };
        entry.count += group.items.length;
        seen.set(group.category, entry);
      }
    }
  }
  return [...seen.values()];
}

/** 当前筛选的区域选项（计划里的全部区域；计数为筛选口径下的未收集数）。 */
export function checklistRegions(
  plan: CollectionPlan,
  filters: PlanFilters,
): { slug: string; label: string; kind: PlanRegion["kind"]; count: number }[] {
  const counts = new Map<string, number>();
  const scoped = buildChecklist(plan, { ...filters, region: "all" });
  for (const region of scoped.regions) {
    counts.set(region.slug, region.remaining);
  }
  return plan.regions.map((region) => ({
    slug: region.slug,
    label: regionDisplayName(region),
    kind: region.kind,
    count: counts.get(region.slug) ?? 0,
  }));
}

/** 导出的固定文件名（后端只接受这两个名字）。 */
export const PLAN_HTML_FILE = "plan.html";
export const PLAN_CSV_FILE = "plan.csv";

export interface PlanExportMeta {
  /** 图鉴版本。 */
  catalogVersion: string;
  /** 生成时间（已格式化，展示用）。 */
  generatedAt: string;
  /** 快照时间（已格式化；分享模式下不写出）。 */
  snapshotTime: string | null;
  /** 输入文件名；分享模式为 null。 */
  inputRef: string | null;
  share: boolean;
  /** 筛选说明（已本地化）。 */
  scopeText: string;
}

/** CSV 单元格：危险前缀加 `'`，必要时加引号（RFC 4180）。 */
export function planCsvCell(value: string): string {
  const dangerous = /^[=+\-@\t\r]/.test(value.trimStart());
  const guarded = dangerous ? `'${value}` : value;
  if (/[",\n\r]/.test(guarded)) {
    return `"${guarded.replace(/"/g, '""')}"`;
  }
  return guarded;
}

function csvRow(cells: string[]): string {
  return cells.map(planCsvCell).join(",");
}

function yesNo(value: boolean): string {
  return value ? t("是", "Yes") : t("否", "No");
}

/** 收集清单 CSV（UTF-8，带 BOM 便于 Excel 识别中文）。 */
export function renderPlanCsv(checklist: PlanChecklist): string {
  const lines: string[] = [];
  lines.push(
    csvRow([
      t("区域", "Region"),
      t("区域类型", "Region type"),
      t("子关卡", "Sub-level"),
      t("类别", "Category"),
      t("名称", "Name"),
      t("内部 ID", "Internal ID"),
      t("口径", "Scope"),
      t("DLC", "DLC"),
      t("底图", "Basemap"),
      t("文字下落", "Text guidance"),
      t("位置线索", "Location hint"),
      t("获取方式", "Acquisition"),
      t("已收取", "Checked"),
    ]),
  );
  for (const region of checklist.regions) {
    for (const unit of region.units) {
      for (const group of unit.groups) {
        for (const item of group.items) {
          lines.push(
            csvRow([
              region.label,
              region.kind === "special" ? t("特殊区域", "Special") : t("主区域", "Main"),
              unit.label,
              group.label,
              item.name,
              item.id,
              requiredLabel(item.required),
              yesNo(item.isDlc),
              yesNo(item.hasBasemap),
              yesNo(item.hasText),
              item.locationHint,
              item.acquisitionText,
              "",
            ]),
          );
        }
      }
    }
  }
  return `﻿${lines.join("\r\n")}\r\n`;
}

/** HTML 转义（导出文件可被浏览器直接打开，动态文本一律转义）。 */

const HTML_STYLE = `
:root { color-scheme: light; }
* { box-sizing: border-box; }
body { margin: 0; padding: 24px; background: #f6f5f2; color: #1c1a17; font: 14px/1.5 "Segoe UI", "Microsoft YaHei", system-ui, sans-serif; }
main { max-width: 960px; margin: 0 auto; background: #fff; border: 1px solid #d9d4ca; border-radius: 8px; padding: 28px 32px; }
h1 { margin: 0 0 4px; font-size: 22px; }
h2 { margin: 28px 0 8px; font-size: 17px; border-bottom: 1px solid #e4dfd5; padding-bottom: 4px; }
h3 { margin: 18px 0 6px; font-size: 14px; color: #4a443c; }
.meta { margin: 0 0 16px; color: #6b645a; font-size: 12px; }
.summary { display: flex; flex-wrap: wrap; gap: 20px; margin: 12px 0 4px; padding: 12px 16px; background: #f4efe2; border: 1px solid #dccfae; border-radius: 6px; }
.summary b { display: block; font-size: 20px; font-variant-numeric: tabular-nums; }
.summary span { font-size: 12px; color: #6b645a; }
table { width: 100%; border-collapse: collapse; margin: 4px 0 8px; }
th, td { border-bottom: 1px solid #ebe6dd; padding: 6px 8px; text-align: left; vertical-align: top; font-size: 13px; }
th { color: #6b645a; font-weight: 600; font-size: 12px; }
td.check { width: 28px; text-align: center; font-size: 16px; color: #7a7368; }
td.id { font-family: Consolas, "Cascadia Mono", monospace; font-size: 12px; color: #6b645a; white-space: nowrap; }
.badge { display: inline-block; margin-left: 6px; padding: 0 6px; border-radius: 999px; border: 1px solid #c9c2b6; color: #6b645a; font-size: 11px; }
.badge.req { border-color: #b9982f; color: #8a6d16; }
.tag { display: inline-block; margin-left: 6px; color: #6b645a; font-size: 11px; }
.empty { color: #6b645a; font-style: italic; }
footer { margin-top: 28px; padding-top: 12px; border-top: 1px solid #e4dfd5; color: #8a8378; font-size: 11px; }
@media print {
  body { background: #fff; padding: 0; }
  main { border: none; border-radius: 0; padding: 0; max-width: none; }
  h2 { page-break-after: avoid; }
  tr { page-break-inside: avoid; }
}
`;

/** 可打印的收集清单 HTML（自包含，无外部资源）。 */
export function renderPlanHtml(checklist: PlanChecklist, meta: PlanExportMeta): string {
  const title = t("古墓丽影：暗影 · 收集计划", "Shadow of the Tomb Raider · Collection plan");
  const scopeNote = t("清单只列出当前筛选下仍未收集的条目。", "Only items still uncollected under the current filters are listed.");
  const metaParts = [
    `${t("图鉴版本", "Catalog")} ${meta.catalogVersion}`,
    `${t("生成时间", "Generated")} ${meta.generatedAt}`,
    meta.inputRef !== null ? `${t("输入", "Input")} ${meta.inputRef}` : null,
    meta.snapshotTime !== null ? `${t("快照", "Snapshot")} ${meta.snapshotTime}` : null,
    meta.share ? t("分享模式：不包含个人路径", "Share mode: no personal paths") : null,
  ].filter((part): part is string => part !== null);

  const rows: string[] = [];
  for (const region of checklist.regions) {
    const kindBadge =
      region.kind === "special" ? `<span class="badge">${escapeHtml(t("特殊区域", "Special"))}</span>` : "";
    rows.push(
      `<h2>${escapeHtml(region.label)}${kindBadge}<span class="tag">${escapeHtml(
        t(`未收集 ${region.remaining}`, `${region.remaining} uncollected`),
      )}</span></h2>`,
    );
    for (const unit of region.units) {
      rows.push(
        `<h3>${escapeHtml(unit.label)}<span class="tag">${escapeHtml(
          t(`${unit.remaining} 项`, `${unit.remaining} item(s)`),
        )}</span></h3>`,
      );
      rows.push("<table><thead><tr>");
      rows.push(`<th></th><th>${escapeHtml(t("名称", "Name"))}</th><th>${escapeHtml(t("类别", "Category"))}</th>`);
      rows.push(`<th>${escapeHtml(t("口径", "Scope"))}</th><th>${escapeHtml(t("下落", "Guidance"))}</th>`);
      rows.push(`<th>${escapeHtml(t("位置线索 / 获取方式", "Location / acquisition"))}</th>`);
      rows.push("</tr></thead><tbody>");
      for (const group of unit.groups) {
        for (const item of group.items) {
          const requiredBadge =
            item.required === true
              ? `<span class="badge req">${escapeHtml(t("必收", "Required"))}</span>`
              : item.required === false
                ? `<span class="badge">${escapeHtml(t("可选", "Optional"))}</span>`
                : `<span class="badge">${escapeHtml(t("未闭合", "Unresolved"))}</span>`;
          const guidance = [
            item.hasBasemap ? t("底图", "Basemap") : null,
            item.hasText ? t("文字", "Text") : null,
          ]
            .filter((part): part is string => part !== null)
            .join(" · ");
          rows.push("<tr>");
          rows.push(`<td class="check">☐</td>`);
          rows.push(
            `<td>${escapeHtml(item.name)}${requiredBadge}${item.isDlc ? `<span class="badge">DLC</span>` : ""}<br><span class="id">${escapeHtml(item.id)}</span></td>`,
          );
          rows.push(`<td>${escapeHtml(item.categoryLabel)}</td>`);
          rows.push(`<td>${escapeHtml(requiredLabel(item.required))}</td>`);
          rows.push(`<td>${escapeHtml(guidance)}</td>`);
          rows.push(
            `<td>${escapeHtml(item.locationHint)}<br><span class="empty">${escapeHtml(item.acquisitionText)}</span></td>`,
          );
          rows.push("</tr>");
        }
      }
      rows.push("</tbody></table>");
    }
  }

  const body =
    checklist.regions.length === 0
      ? `<p class="empty">${escapeHtml(t("当前筛选下没有未收集条目。", "No uncollected items under the current filters."))}</p>`
      : rows.join("\n");

  return `<!DOCTYPE html>
<html lang="${escapeHtml(t("zh-CN", "en"))}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)}</title>
<style>${HTML_STYLE}</style>
</head>
<body>
<main>
<h1>${escapeHtml(title)}</h1>
<p class="meta">${metaParts.map(escapeHtml).join(" · ")}</p>
<section class="summary">
<div><span>${escapeHtml(t("未收集（当前筛选）", "Uncollected (filtered)"))}</span><b>${checklist.items}</b></div>
<div><span>${escapeHtml(t("其中必收", "Required among them"))}</span><b>${checklist.required}</b></div>
<div><span>${escapeHtml(t("涉及区域", "Regions"))}</span><b>${checklist.regionCount}</b></div>
</section>
<p class="meta">${escapeHtml(meta.scopeText)} · ${escapeHtml(scopeNote)}</p>
${body}
<footer>${escapeHtml(
    t(
      "由 SOTTR Explorer 生成。坐标与名称来自项目图鉴数据；底图仅表示该子关卡可打开地图，不代表位置已校准。",
      "Generated by SOTTR Explorer. Names and coordinates come from the project catalog; a basemap means the sub-level has a map, not that positions are calibrated.",
    ),
  )}</footer>
</main>
</body>
</html>
`;
}
