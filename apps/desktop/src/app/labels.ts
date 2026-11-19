// 中文/英文文案与状态标签（SX-015；2026-10-20 增加语言模式）：集中管理，避免散落在模板里导致含义漂移。
//
// 三组概念分开（开发方案“视觉令牌与组件边界”）：
// - 业务状态（observation/assessment）；- 证据依据（evidence_level）；- 定位精度。
// 另有模式/平台上下文说明，不混成单个“可信”标记。
//
// 语言模式：`copy` 使用 getter，读取当前语言；切换语言后模板再次渲染即取到新文案。
import type { AssessedItem, EvidenceLevel, GroupActionKind, LocationPrecision } from "../contract/report";
import { t, tFormatPair, type MessageKey } from "./i18n";

export const copy = {
  get appName(): string {
    return "SOTTR Explorer";
  },
  get subtitle(): string {
    return t("labels.read-only-analysis");
  },
  get mockBanner(): string {
    return t("labels.mock-data-this-page-comes-from-contract");
  },
  get coverage(): string {
    return t("labels.reference-gaps-cover-only-survival-rare-caches");
  },
  get coverageScope(): string {
    return t("labels.reference-gaps-are-the-difference-between-four");
  },
  get noReport(): string {
    return t("labels.this-file-has-not-been-analyzed-yet");
  },
  get noReportHelp(): string {
    return t("labels.pick-a-save-file-from-the-candidate");
  },
  get cancelled(): string {
    return t("labels.cancelled-the-current-view-is-kept");
  },
  get refreshing(): string {
    return t("labels.refreshing-the-content-below-is-the-previous");
  },
  get stale(): string {
    return t("labels.stale-snapshot-it-does-not-reflect");
  },
  get missingUnavailable(): string {
    return t("labels.missing-items-cannot-be-determined-this-time");
  },
  get missingUnavailableHelp(): string {
    return t("labels.the-current-table-is-incomplete");
  },
  get emptyMissing(): string {
    return t("labels.no-reference-gaps-found-within-the-supported");
  },
  get emptyFiltered(): string {
    return t("labels.no-items-match-the-current-filters");
  },
  get emptyUnknownRegion(): string {
    return t("labels.no-collectibles-to-list-check-unclassified-records");
  },
  get unsupportedCell(): string {
    return t("labels.not-supported");
  },
  get unknownCell(): string {
    return t("labels.unknown");
  },
  get countConflict(): string {
    return t("labels.count-needs-review");
  },
  get countConflictHelp(): string {
    return t("labels.the-raw-reading-exceeds-the-community-source");
  },
  get precisionNone(): string {
    return t("catalog.no-location");
  },
  get precisionRegion(): string {
    return t("catalog.region-clue");
  },
  get precisionGroup(): string {
    return t("catalog.guide-group");
  },
  get precisionItem(): string {
    return t("catalog.exact-item");
  },
  get precisionCoordinates(): string {
    return t("catalog.verified-coordinates");
  },
  get noTrustedLocation(): string {
    return t("labels.no-trusted-location-yet");
  },
  get externalLinkNote(): string {
    return t("labels.view-the-stella-guide-in-your-browser");
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
        label: t("labels.likely-missing"),
        tone: "caution",
        presumed: true,
        hint: t("labels.present-in-the-applicable-reference-set-but"),
      };
    case "likely_collected":
      return {
        label: t("labels.likely-collected"),
        tone: "info",
        presumed: true,
        hint: t("labels.observed-in-the-current-table-and-satisfies"),
      };
    case "verified_collected":
      return {
        label: t("labels.verified-collected"),
        tone: "success",
        presumed: false,
        hint: t("labels.verified-in-game-or-against-an-authoritative"),
      };
    case "verified_uncollected":
      return {
        label: t("labels.verified-uncollected"),
        tone: "success",
        presumed: false,
        hint: t("labels.verified-in-game-or-against-an-authoritative"),
      };
    default:
      if (item.observation === "present") {
        return {
          label: t("labels.observed"),
          tone: "info",
          presumed: false,
          hint: t("labels.a-raw-present-value-only-means"),
        };
      }
      return {
        label: t("labels.status-unknown"),
        tone: "neutral",
        presumed: false,
        hint: t("labels.not-enough-evidence-to-determine-collection-status"),
      };
  }
}

export function evidenceLabel(level: EvidenceLevel): string {
  switch (level) {
    case "observed":
      return t("catalog.raw-observation");
    case "community_correlated":
      return t("catalog.community-correlated");
    case "game_validated":
      return t("catalog.in-game-validated");
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

/** 图鉴类别 id → 文案键（与 `data/catalog` 的 categories 一一对应）。 */
const CATEGORY_LABELS: Record<string, MessageKey> = {
  archivist_maps: "category.archivist-maps",
  artifacts: "category.artifacts",
  challenge_tombs: "category.challenge-tombs",
  croft_manor_extra: "category.croft-manor",
  crypt_entrances: "category.crypt-entrances",
  crypt_treasures: "category.crypt-treasures",
  explorer_backpacks: "category.explorer-backpacks",
  gym_training: "category.gym-training",
  monoliths: "category.monoliths",
  rare_resource_caches: "category.rare-resource-caches",
  strongboxes: "category.strongboxes",
  survival_caches: "category.survival-caches",
  test_items: "category.test-items",
};

const SUBCATEGORY_LABELS: Record<string, MessageKey> = {
  relics: "subcategory.relics",
  documents: "subcategory.documents",
  murals: "subcategory.murals",
};

export function categoryLabel(categoryId: string | null, displayName: string | null): string {
  if (categoryId && CATEGORY_LABELS[categoryId]) {
    return t(CATEGORY_LABELS[categoryId]);
  }
  if (displayName) {
    return displayName;
  }
  if (!categoryId) {
    return t("labels.uncategorized");
  }
  return categoryId;
}

/** 子类别 id → 双语名称（故事类别的 Relic / Document / Mural）。 */
export function subcategoryLabel(subcategoryId: string | null): string | null {
  if (!subcategoryId) {
    return null;
  }
  const entry = SUBCATEGORY_LABELS[subcategoryId];
  return entry ? t(entry) : subcategoryId;
}

/** 备选区域名（元数据中没有英文名时使用；与图鉴 `name_zh/name_en` 同一套叫法）。 */
const REGION_LABELS: Record<string, MessageKey> = {
  _challenge_tombs: "region.challenge-tombs",
  _special: "region.special",
  cenote: "region.cenote",
  cozumel: "region.cozumel",
  croft_manor: "region.croft-manor",
  head_of_the_serpent: "region.head-of-the-serpent",
  kuwaq_yaku: "region.kuwaq-yaku",
  mission_of_san_juan: "region.mission-of-san-juan",
  paititi: "region.paititi",
  peruvian_jungle: "region.peruvian-jungle",
  porvenir_oil_fields: "region.porvenir-oil-fields",
  trial_of_the_eagle: "region.trial-of-the-eagle",
};

export function regionLabel(regionId: string | null, displayName: string | null): string {
  if (regionId && REGION_LABELS[regionId]) {
    const entry = REGION_LABELS[regionId];
    return t(entry);
  }
  if (displayName) {
    return displayName;
  }
  if (!regionId) {
    return t("labels.unknown-region");
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
        label: t("labels.whole-group-can-be-checked-against"),
        action: count
          ? tFormatPair(`labels.open-the-guide-for-this-group-s`, { v0: count })
          : t("labels.open-this-group-s-guide-in-your"),
        note: t("labels.the-group-count-is-currently-zero"),
        closed: false,
      };
    case "general_reference":
      return {
        label: t("labels.whole-group-reference-only"),
        action: t("labels.open-this-group-s-guide-in-your"),
        note: t("labels.the-count-is-non-zero-or-unknown"),
        closed: false,
      };
    default:
      return {
        label: t("labels.action-summary-disabled"),
        action: t("labels.read-the-related-sources"),
        note: t("labels.the-current-table-is-incomplete-the-count"),
        closed: true,
      };
  }
}

/** 能力状态文案（诊断与覆盖提示共用）。 */
export function capabilityLabel(status: "supported" | "partial" | "unsupported"): string {
  switch (status) {
    case "supported":
      return t("labels.supported");
    case "partial":
      return t("labels.partial");
    default:
      return t("labels.unsupported");
  }
}

export function severityLabel(severity: "info" | "warning" | "error"): string {
  switch (severity) {
    case "info":
      return t("labels.info");
    case "warning":
      return t("labels.warning");
    default:
      return t("labels.error");
  }
}

/** 已知原因码 → 文案键（未知码保留原文，便于诊断）。 */
const REASON_LABELS: Record<string, MessageKey> = {
  INPUT_UNREADABLE: "reason.INPUT_UNREADABLE",
  INPUT_CHANGED: "reason.INPUT_CHANGED",
  DECOMPRESSION_FAILED: "reason.DECOMPRESSION_FAILED",
  RESOURCE_LIMIT: "reason.RESOURCE_LIMIT",
  UNSUPPORTED_LAYOUT: "reason.UNSUPPORTED_LAYOUT",
  UNSUPPORTED_CONTEXT: "reason.UNSUPPORTED_CONTEXT",
  METADATA_CONFLICT: "reason.METADATA_CONFLICT",
  METADATA_UNAVAILABLE: "reason.METADATA_UNAVAILABLE",
  COUNTER_MISMATCH: "reason.COUNTER_MISMATCH",
  LOCATION_UNVERIFIED: "reason.LOCATION_UNVERIFIED",
  REFERENCE_DIFFERENCE_REQUIRES_COMPLETE_TABLE: "reason.REFERENCE_DIFFERENCE_REQUIRES_COMPLETE_TABLE",
  INVENTORY_INCOMPLETE: "reason.INVENTORY_INCOMPLETE",
  INVENTORY_UNAVAILABLE: "reason.INVENTORY_UNAVAILABLE",
  CONTEXT_NOT_CAMPAIGN: "reason.CONTEXT_NOT_CAMPAIGN",
  SNAPSHOT_NOT_CURRENT: "reason.SNAPSHOT_NOT_CURRENT",
  ZERO_GROUP_COUNT: "reason.ZERO_GROUP_COUNT",
  COMPLETE_TABLE: "reason.COMPLETE_TABLE",
  REFERENCE_MATCH: "reason.REFERENCE_MATCH",
  GROUP_COUNT_NONZERO: "reason.GROUP_COUNT_NONZERO",
  GROUP_COUNT_UNKNOWN: "reason.GROUP_COUNT_UNKNOWN",
  GROUP_COUNT_CONFLICT: "reason.GROUP_COUNT_CONFLICT",
  REFERENCE_NOT_APPLICABLE: "reason.REFERENCE_NOT_APPLICABLE",
  GROUP_ASSOCIATION_UNCONFIRMED: "reason.GROUP_ASSOCIATION_UNCONFIRMED",
};

export function reasonLabel(code: string): string {
  const entry = REASON_LABELS[code];
  return entry ? t(entry) : code;
}

/** 来源 ID → 界面可读名称（未知来源保留原始 ID，便于诊断）。 */
export function sourceLabel(sourceId: string, url: string | null): string {
  const known: Record<string, MessageKey> = {
    "src:stella-saves": "source.stella-saves",
    "src:stella-san-juan": "source.stella-san-juan",
    "src:stella-san-juan-monolith": "source.stella-san-juan-monolith",
    "src:stella-san-juan-crypt": "source.stella-san-juan-crypt",
    "src:trrebootmodtools-dict": "source.trrebootmodtools-dict",
    "src:asl-sottr": "source.asl-sottr",
    "src:plan-doc": "source.plan-doc",
    "src:research-evidence": "source.research-evidence",
  };
  if (known[sourceId]) {
    return t(known[sourceId]);
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
