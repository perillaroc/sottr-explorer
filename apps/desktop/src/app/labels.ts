// 中文文案与状态标签（SX-015）：集中管理，避免散落在模板里导致含义漂移。
//
// 三组概念分开（开发方案“视觉令牌与组件边界”）：
// - 业务状态（observation/assessment）；- 证据依据（evidence_level）；- 定位精度。
// 另有模式/平台上下文说明，不混成单个“可信”标记。
import type { AssessedItem, EvidenceLevel, GroupActionKind, LocationPrecision } from "../contract/report";

export const copy = {
  appName: "SOTTR Explorer",
  subtitle: "只读分析",
  mockBanner:
    "模拟数据：本页来自契约夹具与演示输入，尚未接入真实分析（SX-016 接线）；数值与路径不代表任何真实存档。",
  coverage:
    "参考遗漏仅覆盖普通/稀有贮藏点、石碑、地下墓穴宝藏；其他类别的逐项状态暂不支持。",
  coverageScope:
    "参考遗漏来自四个内部参考类别与当前唯一 ID 集合的差集，不等于全游戏遗漏清单；差集为空也不代表全游戏 100%。",
  noReport: "尚未分析该文件",
  noReportHelp: "选择候选目录中的存档文件并开始分析；原始文件不会被改写。",
  cancelled: "已取消：保留原视图，未产生新报告。",
  refreshing: "正在刷新，以下为上次结果。",
  stale: "旧快照，未反映本次读取。",
  missingUnavailable: "本次无法判断遗漏",
  missingUnavailableHelp: "当前表不完整或参考范围不适用：已关闭参考遗漏推定与据此生成的行动摘要。",
  emptyMissing: "本次支持范围内未发现参考遗漏",
  emptyFiltered: "没有符合当前条件的条目",
  emptyUnknownRegion: "暂无可列的收集品：未分类记录请在诊断中查看",
  unsupportedCell: "暂不支持",
  unknownCell: "未知",
  countConflict: "计数待核对",
  countConflictHelp: "原始读数超出社区来源目标：保留原始值，不裁剪到目标值。",
  precisionNone: "无定位",
  precisionRegion: "区域线索",
  precisionGroup: "攻略组",
  precisionItem: "精确条目",
  precisionCoordinates: "已验证坐标",
  noTrustedLocation: "暂时没有可信定位",
  externalLinkNote: "在浏览器查看 Stella 攻略",
} as const;

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
        label: "推定遗漏",
        tone: "caution",
        presumed: true,
        hint: "在适用参考集合中存在，当前完整物品表未观察到",
      };
    case "likely_collected":
      return {
        label: "推定已收集",
        tone: "info",
        presumed: true,
        hint: "当前表中观察到，且满足参考集合证据规则",
      };
    case "verified_collected":
      return { label: "已验证已收集", tone: "success", presumed: false, hint: "存在游戏内或权威格式验证" };
    case "verified_uncollected":
      return { label: "已验证未收集", tone: "success", presumed: false, hint: "存在游戏内或权威格式验证" };
    default:
      if (item.observation === "present") {
        return {
          label: "已观察",
          tone: "info",
          presumed: false,
          hint: "原始 present 仅表示记录中出现，不自动等于已收集",
        };
      }
      return {
        label: "状态未知",
        tone: "neutral",
        presumed: false,
        hint: "证据不足以判断收集状态",
      };
  }
}

export function evidenceLabel(level: EvidenceLevel): string {
  switch (level) {
    case "observed":
      return "原始观察";
    case "community_correlated":
      return "社区关联";
    case "game_validated":
      return "游戏内验证";
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

export function categoryLabel(categoryId: string | null, displayName: string | null): string {
  if (displayName) {
    return displayName;
  }
  if (!categoryId) {
    return "未分类";
  }
  const known: Record<string, string> = {
    survival_caches: "贮藏点",
    rare_resource_caches: "稀有资源贮藏点",
    monoliths: "石碑",
    crypt_treasures: "地下墓穴宝藏",
  };
  return known[categoryId] ?? categoryId;
}

export function regionLabel(regionId: string | null, displayName: string | null): string {
  if (displayName) {
    return displayName;
  }
  if (!regionId) {
    return "区域未知";
  }
  const known: Record<string, string> = {
    cozumel: "科苏梅尔",
    mission_of_san_juan: "圣胡安",
    paititi: "帕伊提提",
    kuwaq_yaku: "库瓦克雅库",
    peruvian_jungle: "秘鲁丛林",
    cenote: "天然井",
    porvenir_oil_fields: "波文尼尔油田",
    trial_of_the_eagle: "鹰之试炼",
    head_of_the_serpent: "蛇首",
  };
  return known[regionId] ?? regionId;
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
        label: "可按攻略整组检查",
        action: count ? `在浏览器查看本组 ${count} 个条目的攻略` : "在浏览器查看本组攻略",
        note: "整组当前计数为零且范围匹配：可按攻略逐一检查；内部 ID 与攻略编号未逐项对应。",
        closed: false,
      };
    case "general_reference":
      return {
        label: "仅作全组参考",
        action: "在浏览器查看本组攻略",
        note: "计数非零或状态未知：撤销“整组均待查”提示，尚不能指出具体哪几个条目遗漏。",
        closed: false,
      };
    default:
      return {
        label: "行动摘要已关闭",
        action: "阅读相关来源",
        note: "当前表不完整、计数未知或快照过期：关闭遗漏推定与行动摘要；来源仅供一般阅读，不代表当前状态。",
        closed: true,
      };
  }
}

/** 能力状态文案（诊断与覆盖提示共用）。 */
export function capabilityLabel(status: "supported" | "partial" | "unsupported"): string {
  switch (status) {
    case "supported":
      return "可用";
    case "partial":
      return "部分可用";
    default:
      return "不支持";
  }
}

export function severityLabel(severity: "info" | "warning" | "error"): string {
  switch (severity) {
    case "info":
      return "信息";
    case "warning":
      return "警告";
    default:
      return "错误";
  }
}

/** 已知原因码 → 中文（未知码保留原文，便于诊断）。 */
const REASON_LABELS: Record<string, string> = {
  INPUT_UNREADABLE: "文件不可读或正在保存",
  INPUT_CHANGED: "读取期间文件发生变化",
  DECOMPRESSION_FAILED: "解压失败",
  RESOURCE_LIMIT: "超出资源上限",
  UNSUPPORTED_LAYOUT: "布局或段长度不匹配",
  UNSUPPORTED_CONTEXT: "上下文不适用",
  METADATA_CONFLICT: "元数据冲突",
  METADATA_UNAVAILABLE: "元数据不可用",
  COUNTER_MISMATCH: "ID 集合与计数不一致",
  LOCATION_UNVERIFIED: "定位未验证",
  REFERENCE_DIFFERENCE_REQUIRES_COMPLETE_TABLE: "表不完整，参考差集不可用",
  INVENTORY_INCOMPLETE: "物品表不完整",
  INVENTORY_UNAVAILABLE: "物品表不可用",
  CONTEXT_NOT_CAMPAIGN: "上下文不是普通战役",
  SNAPSHOT_NOT_CURRENT: "快照不是当前状态",
  ZERO_GROUP_COUNT: "组计数为零",
  COMPLETE_TABLE: "表恢复完整",
  REFERENCE_MATCH: "参考范围匹配",
  GROUP_COUNT_NONZERO: "组计数非零",
  GROUP_COUNT_UNKNOWN: "组计数未知",
  GROUP_COUNT_CONFLICT: "组计数冲突",
  REFERENCE_NOT_APPLICABLE: "参考集合适用性未确认",
  GROUP_ASSOCIATION_UNCONFIRMED: "组关联未确认",
};

export function reasonLabel(code: string): string {
  return REASON_LABELS[code] ?? code;
}

/** 来源 ID → 界面可读名称（未知来源保留原始 ID，便于诊断）。 */
export function sourceLabel(sourceId: string, url: string | null): string {
  const known: Record<string, string> = {
    "src:stella-saves": "Stella 存档页",
    "src:stella-san-juan": "Stella 攻略（圣胡安）",
    "src:stella-san-juan-monolith": "Stella 攻略（圣胡安石碑）",
    "src:stella-san-juan-crypt": "Stella 攻略（圣胡安墓穴）",
    "src:trrebootmodtools-dict": "TrRebootModTools 字典",
    "src:asl-sottr": "ASL-SOTTR 转录",
    "src:plan-doc": "开发方案（本仓库文档）",
    "src:research-evidence": "研究包证据记录",
  };
  if (known[sourceId]) {
    return known[sourceId];
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
