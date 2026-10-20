// 中文/英文文案与状态标签（SX-015；2026-10-20 增加语言模式）：集中管理，避免散落在模板里导致含义漂移。
//
// 三组概念分开（开发方案“视觉令牌与组件边界”）：
// - 业务状态（observation/assessment）；- 证据依据（evidence_level）；- 定位精度。
// 另有模式/平台上下文说明，不混成单个“可信”标记。
//
// 语言模式：`copy` 使用 getter，读取当前语言；切换语言后模板再次渲染即取到新文案。
import type { AssessedItem, EvidenceLevel, GroupActionKind, LocationPrecision } from "../contract/report";
import { t } from "./i18n";

export const copy = {
  get appName(): string {
    return "SOTTR Explorer";
  },
  get subtitle(): string {
    return t("只读分析", "Read-only analysis");
  },
  get mockBanner(): string {
    return t(
      "模拟数据：本页来自契约夹具与演示输入，尚未接入真实分析（SX-016 接线）；数值与路径不代表任何真实存档。",
      "Mock data: this page comes from contract fixtures and demo inputs, not real analysis (wired in SX-016); values and paths do not represent any real save.",
    );
  },
  get coverage(): string {
    return t(
      "参考遗漏仅覆盖普通/稀有贮藏点、石碑、地下墓穴宝藏；其他类别的逐项状态暂不支持。",
      "Reference gaps cover only survival/rare caches, monoliths and crypt treasures; per-item status is not supported for other categories yet.",
    );
  },
  get coverageScope(): string {
    return t(
      "参考遗漏来自四个内部参考类别与当前唯一 ID 集合的差集，不等于全游戏遗漏清单；差集为空也不代表全游戏 100%。",
      "Reference gaps are the difference between four internal reference categories and the current unique ID set. They are not a full-game missing list, and an empty difference does not mean the whole game is at 100%.",
    );
  },
  get noReport(): string {
    return t("尚未分析该文件", "This file has not been analyzed yet");
  },
  get noReportHelp(): string {
    return t(
      "选择候选目录中的存档文件并开始分析；原始文件不会被改写。",
      "Pick a save file from the candidate folders and start analysis; the original file is never modified.",
    );
  },
  get cancelled(): string {
    return t("已取消：保留原视图，未产生新报告。", "Cancelled: the current view is kept and no new report was produced.");
  },
  get refreshing(): string {
    return t("正在刷新，以下为上次结果。", "Refreshing; the content below is the previous result.");
  },
  get stale(): string {
    return t("旧快照，未反映本次读取。", "Stale snapshot; it does not reflect the latest read.");
  },
  get missingUnavailable(): string {
    return t("本次无法判断遗漏", "Missing items cannot be determined this time");
  },
  get missingUnavailableHelp(): string {
    return t(
      "当前表不完整或参考范围不适用：已关闭参考遗漏推定与据此生成的行动摘要。",
      "The current table is incomplete or the reference scope does not apply: reference-gap inference and the derived action summary are disabled.",
    );
  },
  get emptyMissing(): string {
    return t("本次支持范围内未发现参考遗漏", "No reference gaps found within the supported scope");
  },
  get emptyFiltered(): string {
    return t("没有符合当前条件的条目", "No items match the current filters");
  },
  get emptyUnknownRegion(): string {
    return t("暂无可列的收集品：未分类记录请在诊断中查看", "No collectibles to list; check unclassified records in Diagnostics");
  },
  get unsupportedCell(): string {
    return t("暂不支持", "Not supported");
  },
  get unknownCell(): string {
    return t("未知", "Unknown");
  },
  get countConflict(): string {
    return t("计数待核对", "Count needs review");
  },
  get countConflictHelp(): string {
    return t(
      "原始读数超出社区来源目标：保留原始值，不裁剪到目标值。",
      "The raw reading exceeds the community source target: the raw value is kept and not clipped to the target.",
    );
  },
  get precisionNone(): string {
    return t("无定位", "No location");
  },
  get precisionRegion(): string {
    return t("区域线索", "Region clue");
  },
  get precisionGroup(): string {
    return t("攻略组", "Guide group");
  },
  get precisionItem(): string {
    return t("精确条目", "Exact item");
  },
  get precisionCoordinates(): string {
    return t("已验证坐标", "Verified coordinates");
  },
  get noTrustedLocation(): string {
    return t("暂时没有可信定位", "No trusted location yet");
  },
  get externalLinkNote(): string {
    return t("在浏览器查看 Stella 攻略", "View the Stella guide in your browser");
  },
};

/** 业务状态（observation + assessment 的玩家可读结论）。 */
export interface BusinessStatus {
  label: string;
  tone: "caution" | "info" | "neutral" | "success";
  /** 是否包含“推定”字样（窄窗/概览不得丢失）。 */
  presumed: boolean;
  hint: string;
}

export function businessStatus(item: AssessedItem): BusinessStatus {
  switch (item.assessment) {
    case "likely_missing":
      return {
        label: t("推定遗漏", "Likely missing"),
        tone: "caution",
        presumed: true,
        hint: t(
          "在适用参考集合中存在，当前完整物品表未观察到",
          "Present in the applicable reference set but not observed in the current complete inventory table",
        ),
      };
    case "likely_collected":
      return {
        label: t("推定已收集", "Likely collected"),
        tone: "info",
        presumed: true,
        hint: t(
          "当前表中观察到，且满足参考集合证据规则",
          "Observed in the current table and satisfies the reference-set evidence rule",
        ),
      };
    case "verified_collected":
      return {
        label: t("已验证已收集", "Verified collected"),
        tone: "success",
        presumed: false,
        hint: t("存在游戏内或权威格式验证", "Verified in-game or against an authoritative format"),
      };
    case "verified_uncollected":
      return {
        label: t("已验证未收集", "Verified uncollected"),
        tone: "success",
        presumed: false,
        hint: t("存在游戏内或权威格式验证", "Verified in-game or against an authoritative format"),
      };
    default:
      if (item.observation === "present") {
        return {
          label: t("已观察", "Observed"),
          tone: "info",
          presumed: false,
          hint: t(
            "原始 present 仅表示记录中出现，不自动等于已收集",
            "A raw present value only means the record appears; it does not automatically mean collected",
          ),
        };
      }
      return {
        label: t("状态未知", "Status unknown"),
        tone: "neutral",
        presumed: false,
        hint: t("证据不足以判断收集状态", "Not enough evidence to determine collection status"),
      };
  }
}

export function evidenceLabel(level: EvidenceLevel): string {
  switch (level) {
    case "observed":
      return t("原始观察", "Raw observation");
    case "community_correlated":
      return t("社区关联", "Community-correlated");
    case "game_validated":
      return t("游戏内验证", "In-game validated");
  }
}

/** 逐项评估的依据等级（契约无逐项字段，按规则推导；详情会写出规则 ID）。 */
export function itemEvidenceLevel(item: AssessedItem): EvidenceLevel {
  if (item.assessment.startsWith("verified_")) {
    return "game_validated";
  }
  if (item.assessment.startsWith("likely_")) {
    return "community_correlated";
  }
  return "observed";
}

export function precisionLabel(precision: LocationPrecision | null): string {
  switch (precision) {
    case "region":
      return copy.precisionRegion;
    case "guide_group":
      return copy.precisionGroup;
    case "guide_item":
      return copy.precisionItem;
    case "verified_coordinates":
      return copy.precisionCoordinates;
    default:
      return copy.precisionNone;
  }
}

/** 图鉴类别 id → 双语名称（与 `data/catalog` 的 categories 一一对应）。 */
const CATEGORY_LABELS: Record<string, { zh: string; en: string }> = {
  archivist_maps: { zh: "档案图（可选）", en: "Archivist Maps (optional)" },
  artifacts: { zh: "故事收藏", en: "Artifacts" },
  challenge_tombs: { zh: "挑战古墓", en: "Challenge Tombs" },
  croft_manor_extra: { zh: "克劳馥庄园（序章，不要求）", en: "Croft Manor (Prologue, optional)" },
  crypt_entrances: { zh: "墓穴入口（可选）", en: "Crypt Entrances (optional)" },
  crypt_treasures: { zh: "墓穴宝藏", en: "Crypt Treasures" },
  explorer_backpacks: { zh: "探险者背包（可选）", en: "Explorer Backpacks (optional)" },
  gym_training: { zh: "训练场（不要求）", en: "Gym Training (optional)" },
  monoliths: { zh: "石碑", en: "Monoliths" },
  strongboxes: { zh: "保险箱（可选）", en: "Strongboxes (optional)" },
  survival_caches: { zh: "生存贮藏点", en: "Survival Caches" },
  test_items: { zh: "测试项（不要求）", en: "Test Items (optional)" },
};

const SUBCATEGORY_LABELS: Record<string, { zh: string; en: string }> = {
  relics: { zh: "遗物", en: "Relics" },
  documents: { zh: "文献", en: "Documents" },
  murals: { zh: "壁画", en: "Murals" },
};

export function categoryLabel(categoryId: string | null, displayName: string | null): string {
  if (categoryId && CATEGORY_LABELS[categoryId]) {
    const entry = CATEGORY_LABELS[categoryId];
    return t(entry.zh, entry.en);
  }
  if (displayName) {
    return displayName;
  }
  if (!categoryId) {
    return t("未分类", "Uncategorized");
  }
  return categoryId;
}

/** 子类别 id → 双语名称（故事类别的 Relic / Document / Mural）。 */
export function subcategoryLabel(subcategoryId: string | null): string | null {
  if (!subcategoryId) {
    return null;
  }
  const entry = SUBCATEGORY_LABELS[subcategoryId];
  return entry ? t(entry.zh, entry.en) : subcategoryId;
}

/** 备选区域名（元数据中没有英文名时使用；与图鉴 `name_zh/name_en` 同一套叫法）。 */
const REGION_LABELS: Record<string, { zh: string; en: string }> = {
  _challenge_tombs: { zh: "挑战古墓", en: "CHALLENGE TOMBS" },
  _special: { zh: "特殊 / 跨区域", en: "SPECIAL / CROSS-REGION" },
  cenote: { zh: "灰岩坑", en: "CENOTE" },
  cozumel: { zh: "科祖梅尔", en: "COZUMEL" },
  croft_manor: { zh: "克劳馥庄园", en: "CROFT MANOR" },
  head_of_the_serpent: { zh: "巨蛇之首", en: "HEAD OF THE SERPENT" },
  kuwaq_yaku: { zh: "库阿克雅库", en: "KUWAQ YAKU" },
  mission_of_san_juan: { zh: "圣胡安教会", en: "MISSION OF SAN JUAN" },
  paititi: { zh: "帕依提提", en: "PAITITI" },
  peruvian_jungle: { zh: "秘鲁丛林", en: "PERUVIAN JUNGLE" },
  porvenir_oil_fields: { zh: "波文尼尔石油田", en: "PORVENIR OIL FIELDS" },
  trial_of_the_eagle: { zh: "猎鹰的试炼", en: "TRIAL OF THE EAGLE" },
};

export function regionLabel(regionId: string | null, displayName: string | null): string {
  if (regionId && REGION_LABELS[regionId]) {
    const entry = REGION_LABELS[regionId];
    return t(entry.zh, entry.en);
  }
  if (displayName) {
    return displayName;
  }
  if (!regionId) {
    return t("区域未知", "Unknown region");
  }
  return regionId;
}

/** 组级门禁（来自领域层结论；界面不凭计数自行推导）。 */
export interface GateText {
  label: string;
  action: string;
  note: string;
  /** true 表示关闭行动摘要（unavailable）。 */
  closed: boolean;
}

export function gateText(kind: GroupActionKind, targetCount: number | null): GateText {
  const count = targetCount ?? null;
  switch (kind) {
    case "whole_group_check":
      return {
        label: t("可按攻略整组检查", "Whole group can be checked against the guide"),
        action: count
          ? t(
              `在浏览器查看本组 ${count} 个条目的攻略`,
              `Open the guide for this group's ${count} items in your browser`,
            )
          : t("在浏览器查看本组攻略", "Open this group's guide in your browser"),
        note: t(
          "整组当前计数为零且范围匹配：可按攻略逐一检查；内部 ID 与攻略编号未逐项对应。",
          "The group count is currently zero and the scope matches: check it item by item with the guide; internal IDs are not mapped one-to-one to guide numbers.",
        ),
        closed: false,
      };
    case "general_reference":
      return {
        label: t("仅作全组参考", "Whole-group reference only"),
        action: t("在浏览器查看本组攻略", "Open this group's guide in your browser"),
        note: t(
          "计数非零或状态未知：撤销“整组均待查”提示，尚不能指出具体哪几个条目遗漏。",
          "The count is non-zero or unknown: the “whole group unchecked” hint is withdrawn, and specific missing items cannot be identified yet.",
        ),
        closed: false,
      };
    default:
      return {
        label: t("行动摘要已关闭", "Action summary disabled"),
        action: t("阅读相关来源", "Read the related sources"),
        note: t(
          "当前表不完整、计数未知或快照过期：关闭遗漏推定与行动摘要；来源仅供一般阅读，不代表当前状态。",
          "The current table is incomplete, the count is unknown, or the snapshot is stale: gap inference and the action summary are disabled; sources are for general reading only and do not represent the current state.",
        ),
        closed: true,
      };
  }
}

/** 能力状态文案（诊断与覆盖提示共用）。 */
export function capabilityLabel(status: "supported" | "partial" | "unsupported"): string {
  switch (status) {
    case "supported":
      return t("可用", "Supported");
    case "partial":
      return t("部分可用", "Partial");
    default:
      return t("不支持", "Unsupported");
  }
}

export function severityLabel(severity: "info" | "warning" | "error"): string {
  switch (severity) {
    case "info":
      return t("信息", "Info");
    case "warning":
      return t("警告", "Warning");
    default:
      return t("错误", "Error");
  }
}

/** 已知原因码 → 双语（未知码保留原文，便于诊断）。 */
const REASON_LABELS: Record<string, { zh: string; en: string }> = {
  INPUT_UNREADABLE: { zh: "文件不可读或正在保存", en: "File unreadable or still being saved" },
  INPUT_CHANGED: { zh: "读取期间文件发生变化", en: "File changed during reading" },
  DECOMPRESSION_FAILED: { zh: "解压失败", en: "Decompression failed" },
  RESOURCE_LIMIT: { zh: "超出资源上限", en: "Resource limit exceeded" },
  UNSUPPORTED_LAYOUT: { zh: "布局或段长度不匹配", en: "Layout or section length mismatch" },
  UNSUPPORTED_CONTEXT: { zh: "上下文不适用", en: "Context not applicable" },
  METADATA_CONFLICT: { zh: "元数据冲突", en: "Metadata conflict" },
  METADATA_UNAVAILABLE: { zh: "元数据不可用", en: "Metadata unavailable" },
  COUNTER_MISMATCH: { zh: "ID 集合与计数不一致", en: "ID set and count disagree" },
  LOCATION_UNVERIFIED: { zh: "定位未验证", en: "Location unverified" },
  REFERENCE_DIFFERENCE_REQUIRES_COMPLETE_TABLE: {
    zh: "表不完整，参考差集不可用",
    en: "Table incomplete; reference difference unavailable",
  },
  INVENTORY_INCOMPLETE: { zh: "物品表不完整", en: "Inventory table incomplete" },
  INVENTORY_UNAVAILABLE: { zh: "物品表不可用", en: "Inventory table unavailable" },
  CONTEXT_NOT_CAMPAIGN: { zh: "上下文不是普通战役", en: "Context is not the campaign" },
  SNAPSHOT_NOT_CURRENT: { zh: "快照不是当前状态", en: "Snapshot is not current" },
  ZERO_GROUP_COUNT: { zh: "组计数为零", en: "Group count is zero" },
  COMPLETE_TABLE: { zh: "表恢复完整", en: "Table recovered completely" },
  REFERENCE_MATCH: { zh: "参考范围匹配", en: "Reference scope matches" },
  GROUP_COUNT_NONZERO: { zh: "组计数非零", en: "Group count is non-zero" },
  GROUP_COUNT_UNKNOWN: { zh: "组计数未知", en: "Group count unknown" },
  GROUP_COUNT_CONFLICT: { zh: "组计数冲突", en: "Group count conflict" },
  REFERENCE_NOT_APPLICABLE: { zh: "参考集合适用性未确认", en: "Reference-set applicability unconfirmed" },
  GROUP_ASSOCIATION_UNCONFIRMED: { zh: "组关联未确认", en: "Group association unconfirmed" },
};

export function reasonLabel(code: string): string {
  const entry = REASON_LABELS[code];
  return entry ? t(entry.zh, entry.en) : code;
}

/** 来源 ID → 界面可读名称（未知来源保留原始 ID，便于诊断）。 */
export function sourceLabel(sourceId: string, url: string | null): string {
  const known: Record<string, { zh: string; en: string }> = {
    "src:stella-saves": { zh: "Stella 存档页", en: "Stella save page" },
    "src:stella-san-juan": { zh: "Stella 攻略（圣胡安）", en: "Stella guide (San Juan)" },
    "src:stella-san-juan-monolith": { zh: "Stella 攻略（圣胡安石碑）", en: "Stella guide (San Juan monolith)" },
    "src:stella-san-juan-crypt": { zh: "Stella 攻略（圣胡安墓穴）", en: "Stella guide (San Juan crypt)" },
    "src:trrebootmodtools-dict": { zh: "TrRebootModTools 字典", en: "TrRebootModTools dictionary" },
    "src:asl-sottr": { zh: "ASL-SOTTR 转录", en: "ASL-SOTTR transcript" },
    "src:plan-doc": { zh: "开发方案（本仓库文档）", en: "Development plan (repository docs)" },
    "src:research-evidence": { zh: "研究包证据记录", en: "Research evidence record" },
  };
  if (known[sourceId]) {
    return t(known[sourceId].zh, known[sourceId].en);
  }
  if (url) {
    try {
      return new URL(url).host;
    } catch {
      return sourceId;
    }
  }
  return sourceId;
}
