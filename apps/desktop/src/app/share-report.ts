// 分享报告（P1-5）：自包含、离线可读的进度 HTML。
//
// 边界：
// - 只汇总当前报告与图鉴计划，不新增判定；完成度来自报告结论（推定）。
// - 分享口径固定：不含存档路径、用户信息、输入摘要/哈希与快照时间。
// - 不包含外部资源（CSS/JS/图片/字体），也不包含地图缩略图：vi7ual 素材的
//   非商业使用边界（D7）未决策前，报告只给数字与条目清单。
// - 条目名称与区域名来自图鉴，动态文本一律转义。

import type { Report } from "../contract/report";
import type { CollectionPlan } from "../data/atlas";
import { escapeHtml } from "./html";
import { localizedName, t, tFormatPair } from "./i18n";
import { categoryLabel, subcategoryLabel } from "./labels";

export const SHARE_REPORT_FILE = "share.html";

export interface ShareReportMeta {
  /** 图鉴数据版本。 */
  catalogVersion: string;
  /** 生成时间（展示用；不含时区外的本机信息）。 */
  generatedAt: string;
}

export interface ShareRegionRow {
  slug: string;
  label: string;
  kind: "main" | "special";
  collected: number;
  total: number;
  requiredCollected: number;
  requiredTotal: number;
}

export interface ShareMissingItem {
  id: string;
  name: string;
  categoryLabel: string;
}

export interface ShareMissingGroup {
  regionSlug: string;
  regionLabel: string;
  items: ShareMissingItem[];
}

export interface ShareReportModel {
  mainCollected: number;
  mainTotal: number;
  requiredCollected: number;
  requiredTotal: number;
  missingRequired: number;
  overallPercent: number | null;
  overallReason: string | null;
  regions: ShareRegionRow[];
  missing: ShareMissingGroup[];
}

/** 从计划与报告提取分享报告所需的数字（纯函数，便于测试）。 */
export function shareReportModel(plan: CollectionPlan, report: Report): ShareReportModel {
  let requiredCollected = 0;
  let requiredTotal = 0;
  const regions: ShareRegionRow[] = [];
  const missing: ShareMissingGroup[] = [];

  for (const region of plan.regions) {
    let regionRequiredCollected = 0;
    let regionRequiredTotal = 0;
    const missingItems: ShareMissingItem[] = [];
    for (const group of region.groups) {
      const groupLabel = group.subcategory
        ? (subcategoryLabel(group.subcategory) ?? group.name)
        : categoryLabel(group.category, group.name);
      for (const item of group.items) {
        if (item.required_for_100 !== true) {
          continue;
        }
        regionRequiredTotal += 1;
        requiredTotal += 1;
        if (item.status === "collected") {
          regionRequiredCollected += 1;
          requiredCollected += 1;
        } else {
          missingItems.push({ id: item.id, name: item.display_name, categoryLabel: groupLabel });
        }
      }
    }
    regions.push({
      slug: region.slug,
      label: localizedName(region.name_zh, region.name_en, region.slug),
      kind: region.kind,
      collected: region.collected,
      total: region.total,
      requiredCollected: regionRequiredCollected,
      requiredTotal: regionRequiredTotal,
    });
    if (missingItems.length > 0) {
      missing.push({
        regionSlug: region.slug,
        regionLabel: localizedName(region.name_zh, region.name_en, region.slug),
        items: missingItems,
      });
    }
  }

  const overall = report.observations.overall_completion;
  return {
    mainCollected: plan.main_collected,
    mainTotal: plan.main_total,
    requiredCollected,
    requiredTotal,
    missingRequired: requiredTotal - requiredCollected,
    overallPercent: overall.value,
    overallReason: overall.value === null ? (overall.reason ?? null) : null,
    regions,
    missing,
  };
}

const STYLE = `
:root { color-scheme: light; }
* { box-sizing: border-box; }
body { margin: 0; padding: 24px; background: #f6f5f2; color: #1c1a17; font: 14px/1.55 "Segoe UI", "Microsoft YaHei", system-ui, sans-serif; }
main { max-width: 900px; margin: 0 auto; background: #fff; border: 1px solid #d9d4ca; border-radius: 8px; padding: 28px 32px; }
h1 { margin: 0 0 4px; font-size: 22px; }
h2 { margin: 26px 0 8px; font-size: 16px; border-bottom: 1px solid #e4dfd5; padding-bottom: 4px; }
.meta { margin: 0 0 14px; color: #6b645a; font-size: 12px; }
.cards { display: flex; flex-wrap: wrap; gap: 16px; margin: 14px 0 4px; }
.card { flex: 1 1 150px; padding: 12px 14px; background: #f4efe2; border: 1px solid #dccfae; border-radius: 6px; }
.card b { display: block; font-size: 21px; font-variant-numeric: tabular-nums; }
.card span { font-size: 12px; color: #6b645a; }
table { width: 100%; border-collapse: collapse; margin: 4px 0 8px; }
th, td { border-bottom: 1px solid #ebe6dd; padding: 6px 8px; text-align: left; vertical-align: top; font-size: 13px; }
th { color: #6b645a; font-weight: 600; font-size: 12px; }
td.num { text-align: right; font-variant-numeric: tabular-nums; white-space: nowrap; }
.badge { display: inline-block; margin-left: 6px; padding: 0 6px; border-radius: 999px; border: 1px solid #c9c2b6; color: #6b645a; font-size: 11px; }
.id { font-family: Consolas, "Cascadia Mono", monospace; font-size: 12px; color: #6b645a; white-space: nowrap; }
.empty { color: #6b645a; font-style: italic; }
footer { margin-top: 28px; padding-top: 12px; border-top: 1px solid #e4dfd5; color: #8a8378; font-size: 11px; }
footer p { margin: 3px 0; }
@media print {
  body { background: #fff; padding: 0; }
  main { border: none; border-radius: 0; padding: 0; max-width: none; }
  h2 { page-break-after: avoid; }
  tr { page-break-inside: avoid; }
}
`;

/** 生成自包含的分享报告 HTML（无外部资源；不含路径/用户名/哈希）。 */
export function renderShareReport(
  plan: CollectionPlan,
  report: Report,
  meta: ShareReportMeta,
): string {
  const model = shareReportModel(plan, report);
  const title = t("shareReport.shadow-of-the-tomb-raider-progress-report");

  const overallText =
    model.overallPercent === null
      ? t("shareReport.unavailable")
      : `${model.overallPercent.toFixed(2)}%`;
  const overallNote =
    model.overallReason === null ? t("shareReport.estimated-community-calibrated-candidate") : model.overallReason;

  const regionRows = model.regions
    .map((region) => {
      const badge =
        region.kind === "special" ? `<span class="badge">${escapeHtml(t("common.special"))}</span>` : "";
      return `<tr><td>${escapeHtml(region.label)}${badge}</td><td class="num">${region.collected} / ${region.total}</td><td class="num">${region.requiredCollected} / ${region.requiredTotal}</td></tr>`;
    })
    .join("\n");

  const missingBlocks =
    model.missing.length === 0
      ? `<p class="empty">${escapeHtml(t("shareReport.all-required-items-have-been-observed"))}</p>`
      : model.missing
          .map((group) => {
            const rows = group.items
              .map(
                (item) =>
                  `<tr><td>${escapeHtml(item.name)}</td><td>${escapeHtml(item.categoryLabel)}</td><td class="id">${escapeHtml(item.id)}</td></tr>`,
              )
              .join("\n");
            return `<h2>${escapeHtml(group.regionLabel)}<span class="badge">${escapeHtml(
              tFormatPair(`shareReport.missing`, { v0: group.items.length }),
            )}</span></h2>
<table><thead><tr><th>${escapeHtml(t("planChecklist.name"))}</th><th>${escapeHtml(
              t("common.category"),
            )}</th><th>${escapeHtml(t("planChecklist.internal-id"))}</th></tr></thead><tbody>
${rows}
</tbody></table>`;
          })
          .join("\n");

  return `<!DOCTYPE html>
<html lang="${escapeHtml(t("planChecklist.en"))}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)}</title>
<style>${STYLE}</style>
</head>
<body>
<main>
<h1>${escapeHtml(title)}</h1>
<p class="meta">${escapeHtml(t("planChecklist.catalog"))} ${escapeHtml(meta.catalogVersion)} · ${escapeHtml(
    t("planChecklist.generated"),
  )} ${escapeHtml(meta.generatedAt)}</p>

<section class="cards">
<div class="card"><span>${escapeHtml(t("shareReport.main-region-collectibles"))}</span><b>${model.mainCollected} / ${model.mainTotal}</b></div>
<div class="card"><span>${escapeHtml(t("common.required"))}</span><b>${model.requiredCollected} / ${model.requiredTotal}</b></div>
<div class="card"><span>${escapeHtml(t("shareReport.required-missing"))}</span><b>${model.missingRequired}</b></div>
<div class="card"><span>${escapeHtml(t("shareReport.in-game-completion"))}</span><b>${escapeHtml(overallText)}</b><span>${escapeHtml(overallNote)}</span></div>
</section>

<h2>${escapeHtml(t("shareReport.region-progress"))}</h2>
<table><thead><tr><th>${escapeHtml(t("common.region"))}</th><th>${escapeHtml(
    t("shareReport.collected-total"),
  )}</th><th>${escapeHtml(t("common.required"))}</th></tr></thead><tbody>
${regionRows}
</tbody></table>

<h2>${escapeHtml(t("shareReport.required-missing-list"))}</h2>
${missingBlocks}

<footer>
<p>${escapeHtml(
    t("shareReport.generated-by-sottr-explorer-read-only-analysis"),
  )}</p>
<p>${escapeHtml(
    t("shareReport.names-regions-and-coordinates-come-from"),
  )}</p>
</footer>
</main>
</body>
</html>
`;
}
