// SX-003 类型安全消费示例：验证冻结契约可被前端直接消费。
// 仅参与类型检查（不进入运行入口、不发起任何请求）；随 pnpm build / contract:check 校验。
import type { Report } from "./report";
import type { DesktopResult } from "./desktop-result";
import type { MetadataDataset, MetadataItem } from "./metadata";

/** 名称回退：display_name 未知时使用带标识的类别加 ID 回退，不自造正式名称。 */
export function displayNameOf(item: MetadataItem): string {
  if (item.display_name !== null && item.name_kind !== "unknown") {
    return item.display_name;
  }
  const category = item.category_id ?? "未知类别";
  return `${category}（名称待确认）· ${item.internal_id}`;
}

/** 参考遗漏推定列表；原始 present 不会被当作已收集。 */
export function likelyMissingIds(report: Report): string[] {
  return report.assessments
    .filter((entry) => entry.assessment === "likely_missing")
    .map((entry) => entry.internal_id);
}

/** 可按攻略整组检查的组：门禁由领域层给出，前端不凭计数文案自行下结论。 */
export function wholeGroupCheckIds(report: Report): string[] {
  return report.locations
    .filter((location) => location.group_actionability?.kind === "whole_group_check")
    .map((location) => location.guide_group_id ?? location.location_id);
}

/** 业务状态、证据等级与时效状态分开消费，不合并为单一徽标。 */
export function summarize(result: DesktopResult) {
  const coverageNotice = "参考遗漏仅覆盖普通/稀有贮藏点、石碑、地下墓穴宝藏";
  if (result.report === null) {
    return { freshness: result.freshness, state: "无结果", evidence: "unknown", coverageNotice };
  }
  return {
    freshness: result.freshness,
    state: result.report.observations.overall_completion.support_status,
    evidence: result.report.observations.overall_completion.evidence_level,
    coverageNotice,
  };
}

/** 名称种类统计（联合类型可穷举）。 */
export function nameKindSummary(dataset: MetadataDataset): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const item of Object.values(dataset.items)) {
    counts[item.name_kind] = (counts[item.name_kind] ?? 0) + 1;
  }
  return counts;
}
