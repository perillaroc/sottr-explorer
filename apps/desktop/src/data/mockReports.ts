// 模拟报告（SX-015）：以契约夹具与演示数据构造的 Report/DesktopResult。
//
// 所有数值都是模拟数据（界面显著标注），只用于演示状态与证据呈现规则：
// - 完整样例：整体候选 66.20%、参考遗漏 12 项（取自开发方案公开样本描述）、
//   圣胡安三个攻略组 0/8、0/1、0/1 均为 whole_group_check；
// - 非零/降级样例：直接使用 SX-003 契约夹具（报告形状由 schema 与 Rust 测试保证）；
// - 目标不是“编造存档真相”，而是让界面在合法、降级、错误、过期、取消等状态下
//   都能被组件测试与人工验收检查。
import type { AssessedItem, Evidence, LocationHint, RegionCounter, Report } from "../contract/report";
import type { InputError } from "./source";
import { counterKey, demoMetadata, REFERENCE_CATEGORY_ORDER } from "./metadata";
import { degradedReport as degradedFixture, nonzeroReport as nonzeroFixture } from "./fixtures";

/** 报告种子：模拟的输入摘要（与候选文件对应，便于展示快照身份）。 */
export interface ReportSeed {
  compressedSha256: string;
  compressedBytes: number;
}

const GROUP_REFERENCE_SET = "rs:stella-100pct-common";

/** 开发方案公开的 12 项参考遗漏样本（内部 ID → 内部名称片段见方案“具体物品清单”）。 */
export const DEMO_MISSING_IDS = [
  "0x439B19A6",
  "0xD8CD8257",
  "0x0F28B6EC",
  "0x0F28B6ED",
  "0x0F28B6EE",
  "0x0F28B6EF",
  "0x851B4915",
  "0xAE1A2E7A",
  "0xB419E6FE",
  "0xFF9EAC6E",
  "0xFF9EAC79",
  "0xBA3B0D06",
] as const;

/** 已观察（present）示例：参考集合内条目默认只表示已观察，不给收集状态。 */
export const DEMO_OBSERVED_IDS = ["0x3672536E", "0xDD26A1BC"] as const;

/** 未分类/未知 ID 示例：保留原始观察，不计入收集品数。 */
export const DEMO_UNCLASSIFIED_IDS = ["0x11111111", "0x99999999"] as const;

function inputDigest(seed: ReportSeed) {
  return {
    compressed_sha256: seed.compressedSha256,
    expanded_sha256: null,
    compressed_bytes: seed.compressedBytes,
    expanded_bytes: null,
  };
}

function counter(
  regionId: string,
  categoryId: string,
  value: number,
  target: number,
  evidenceRef: string,
): RegionCounter {
  return {
    counter_key: counterKey(regionId, categoryId),
    region_id: regionId,
    category_id: categoryId,
    value,
    unit: "count",
    support_status: "supported",
    reason: null,
    evidence_level: "community_correlated",
    evidence_refs: [evidenceRef],
    limitations: ["来源目标来自社区脚本，未证明适用所有版本/DLC（模拟数据）"],
    source_target: target,
    within_source_target: value <= target,
    raw_value: value,
    raw_value_hex: `0x${value.toString(16).toUpperCase().padStart(8, "0")}`,
  };
}

function groupLocation(
  guideGroupId: string,
  members: string[],
  targetCount: number,
  digest: ReturnType<typeof inputDigest>,
  evidenceRef: string,
): LocationHint {
  const group = demoMetadata.guide_groups[guideGroupId];
  return {
    location_id: `loc:${guideGroupId}`,
    precision: "guide_group",
    region_id: group?.region_id ?? null,
    guide_group_id: guideGroupId,
    guide_item: null,
    member_ids: members,
    target_count: targetCount,
    mapping_kind: "guide_group_membership",
    source_refs: group?.source_refs ?? [],
    reference_set_version: "1.0.0",
    applicable_mode: "campaign",
    applicable_platform: null,
    coordinates: null,
    map_transform: null,
    group_actionability: {
      kind: "whole_group_check",
      reason_codes: ["COMPLETE_TABLE", "ZERO_GROUP_COUNT", "REFERENCE_MATCH"],
      evidence_refs: [evidenceRef],
      evaluated_input_digest: digest,
    },
    limitations: [
      "未建立内部 ID 与攻略编号的一对一映射；“按攻略逐一检查”基于整组当前计数为零与参考范围匹配（模拟数据）",
    ],
    reason: null,
  };
}

/** 完整且支持的报告（模拟 save1.dat）。 */
export function buildCompleteReport(seed: ReportSeed): Report {
  const digest = inputDigest(seed);
  const sanJuanCaches = counter("mission_of_san_juan", "survival_caches", 0, 8, "ev:counter:san-juan:caches");
  const sanJuanMonoliths = counter("mission_of_san_juan", "monoliths", 0, 1, "ev:counter:san-juan:monoliths");
  const sanJuanCrypts = counter("mission_of_san_juan", "crypts", 0, 1, "ev:counter:san-juan:crypts");
  const paititiCaches = counter("paititi", "survival_caches", 31, 34, "ev:counter:paititi:caches");

  const assessments: AssessedItem[] = [
    ...DEMO_MISSING_IDS.map((id) => ({
      internal_id: id,
      observation: "reference_present_current_absent" as const,
      assessment: "likely_missing" as const,
      rule_id: "refdiff.v1.likely_missing",
      evidence_refs: ["ev:refdiff:rs:stella-100pct-common"],
      limitations: ["仅限四个内部参考类别；不等于全游戏遗漏清单（模拟数据）"],
    })),
    ...DEMO_OBSERVED_IDS.map((id) => ({
      internal_id: id,
      observation: "present" as const,
      assessment: "unknown" as const,
      rule_id: null,
      evidence_refs: ["ev:record:prefix:0"],
      limitations: ["原始 present 默认仅表示已观察，不自动等于已收集（模拟数据）"],
    })),
    ...DEMO_UNCLASSIFIED_IDS.map((id) => ({
      internal_id: id,
      observation: "present" as const,
      assessment: "unknown" as const,
      rule_id: null,
      evidence_refs: [],
      limitations: ["未分类记录，不计入收集品数（模拟数据）"],
    })),
    {
      internal_id: "0x5EED0001",
      observation: "unknown" as const,
      assessment: "unknown" as const,
      rule_id: null,
      evidence_refs: [],
      limitations: ["证据不足以判断收集状态（模拟数据）"],
    },
  ];

  const cachesGroup = demoMetadata.guide_groups["stella:san-juan-caches"];
  const monolithGroup = demoMetadata.guide_groups["stella:san-juan-monolith"];
  const cryptGroup = demoMetadata.guide_groups["stella:san-juan-crypt"];

  const locations: LocationHint[] = [
    groupLocation("stella:san-juan-caches", cachesGroup?.member_ids ?? [], 8, digest, "ev:counter:san-juan:caches"),
    groupLocation("stella:san-juan-monolith", monolithGroup?.member_ids ?? [], 1, digest, "ev:counter:san-juan:monoliths"),
    groupLocation("stella:san-juan-crypt", cryptGroup?.member_ids ?? [], 1, digest, "ev:counter:san-juan:crypts"),
    {
      location_id: "loc:region:paititi",
      precision: "region",
      region_id: "paititi",
      guide_group_id: null,
      guide_item: null,
      member_ids: ["0x851B4915", "0xB419E6FE"],
      target_count: null,
      mapping_kind: "region_clue",
      source_refs: ["src:plan-doc", "src:research-evidence"],
      reference_set_version: null,
      applicable_mode: "campaign",
      applicable_platform: null,
      coordinates: null,
      map_transform: null,
      group_actionability: null,
      limitations: ["仅有区域线索，无攻略组或精确条目对应（模拟数据）"],
      reason: null,
    },
  ];

  const evidence: Evidence[] = [
    {
      evidence_id: "ev:overall:0x28",
      method: "float32_le read at fixed offset",
      source_refs: ["src:stella-saves"],
      sample_scope: "private_4mib_family_5_files（模拟）",
      limitations: ["社区校准候选，未经游戏内独立核实"],
      offset: 40,
      offset_hex: "0x28",
      record_index: null,
      reference_set_id: null,
      cross_checks: ["公开 100% 存档在 0x28 处为 100.0"],
      source_versions: {},
    },
    {
      evidence_id: "ev:refdiff:rs:stella-100pct-common",
      method: "set difference against bounded reference set",
      source_refs: ["src:stella-saves"],
      sample_scope: "two_author_labeled_100pct_slots（模拟）",
      limitations: ["仅四个内部参考类别", "差集为空不等于全游戏收集完成"],
      offset: null,
      offset_hex: null,
      record_index: null,
      reference_set_id: GROUP_REFERENCE_SET,
      cross_checks: ["集合差集不使用字典全集"],
      source_versions: { "ASL-SOTTR": "b758ee890eac30d9e3b3477999a30f8f9e442f00" },
    },
    {
      evidence_id: "ev:counter:san-juan:caches",
      method: "region counter read（0x10BC + 行号 × 104 + 相对偏移）",
      source_refs: ["src:asl-sottr"],
      sample_scope: "community_counter_mapping（模拟）",
      limitations: ["来源目标为社区脚本值"],
      offset: 4600,
      offset_hex: "0x11F8",
      record_index: null,
      reference_set_id: null,
      cross_checks: [],
      source_versions: {},
    },
    {
      evidence_id: "ev:record:prefix:0",
      method: "prefix u32 pair read",
      source_refs: ["src:trrebootmodtools-dict"],
      sample_scope: "4mib_family_prefix（模拟）",
      limitations: ["ID 出现不等于拥有状态"],
      offset: 68,
      offset_hex: "0x44",
      record_index: 0,
      reference_set_id: null,
      cross_checks: [],
      source_versions: {},
    },
  ];

  return {
    schema_version: "1.0.0",
    tool_version: "0.1.0-mock",
    parser_profile_id: "sottr-save-family-4mib-v1",
    metadata_version: demoMetadata.metadata_version,
    reference_set_version: "1.0.0",
    input_digest: digest,
    context: {
      platform: "windows_steam",
      mode: "campaign",
      build: null,
      dlc: null,
      source: "user_declared",
      limitations: ["模式为用户声明，不是格式识别证明", "本次为模拟数据（SX-015），尚未接入真实分析"],
    },
    capabilities: {
      container: { status: "supported", reason_codes: [] },
      sections: { status: "supported", reason_codes: [] },
      inventory: { status: "supported", reason_codes: [] },
      counters: { status: "supported", reason_codes: [] },
      reference_difference: { status: "supported", reason_codes: [] },
      location: { status: "partial", reason_codes: ["LOCATION_UNVERIFIED"] },
    },
    coverage: {
      supported_category_ids: [...REFERENCE_CATEGORY_ORDER],
      reference_category_ids: [...REFERENCE_CATEGORY_ORDER],
      supported_region_counter_keys: [
        sanJuanCaches.counter_key,
        sanJuanMonoliths.counter_key,
        sanJuanCrypts.counter_key,
        paititiCaches.counter_key,
      ],
      unavailable_reason_codes: [],
      evidence_refs: ["ev:refdiff:rs:stella-100pct-common"],
    },
    observations: {
      container: {
        status: "supported",
        format: "zlib",
        valid: true,
        eof: true,
        compressed_bytes: seed.compressedBytes,
        expanded_bytes: 4_194_304,
        trailing_bytes: 0,
        error: null,
      },
      sections: {
        status: "supported",
        valid: true,
        byte_length: 11_892,
        byte_length_hex: "0x2E74",
        start_offset: 49_108,
        start_offset_hex: "0xBFD4",
        section_count: 27,
        error: null,
      },
      overall_completion: {
        value: 66.20481872558594,
        unit: "percent",
        support_status: "supported",
        reason: null,
        evidence_level: "community_correlated",
        evidence_refs: ["ev:overall:0x28"],
        limitations: ["社区校准候选，未经游戏内独立核实（模拟数据）"],
      },
      inventory: {
        status: "supported",
        inventory_complete: true,
        completeness_scope: "parser_profile 前缀与有效 type28 恢复完整（模拟数据）；不代表全游戏全集完备",
        records: [
          {
            table: "prefix",
            index: 0,
            internal_id: "0x3672536E",
            raw_companion_u32: 1,
            raw_companion_hex: "0x00000001",
            raw_low_two_bits: 1,
            id_offset: 68,
            id_offset_hex: "0x44",
          },
          {
            table: "prefix",
            index: 1,
            internal_id: "0x11111111",
            raw_companion_u32: 3,
            raw_companion_hex: "0x00000003",
            raw_low_two_bits: 3,
            id_offset: 76,
            id_offset_hex: "0x4C",
          },
          {
            table: "type28_overflow",
            index: 0,
            internal_id: "0xDD26A1BC",
            raw_companion_u32: 1,
            raw_companion_hex: "0x00000001",
            raw_low_two_bits: 1,
            id_offset: 49_124,
            id_offset_hex: "0xBFE4",
          },
        ],
        record_count: 3,
        unique_id_count: 3,
        duplicate_record_count: 0,
      },
      region_counters: [sanJuanCaches, sanJuanMonoliths, sanJuanCrypts, paititiCaches],
    },
    assessments,
    locations,
    evidence,
    diagnostics: [
      {
        code: "LOCATION_UNVERIFIED",
        severity: "info",
        scope: "location",
        message: "已验证地图坐标 0 项，界面不得绘制精确点位（模拟数据）",
        evidence_refs: ["ev:refdiff:rs:stella-100pct-common"],
      },
      {
        code: "MODE_USER_DECLARED",
        severity: "info",
        scope: "input",
        message: "普通战役上下文来自用户声明，不是格式识别证明",
        evidence_refs: [],
      },
      {
        code: "MOCK_DATA",
        severity: "warning",
        scope: "input",
        message: "本报告为 SX-015 模拟数据；真实分析接线在 SX-016",
        evidence_refs: [],
      },
    ],
  };
}

/** 资料文件（profile.dat）：只做容器观察，不进入战役解释。 */
export function buildProfileReport(seed: ReportSeed): Report {
  const digest = inputDigest(seed);
  return {
    schema_version: "1.0.0",
    tool_version: "0.1.0-mock",
    parser_profile_id: "sottr-save-family-4mib-v1",
    metadata_version: demoMetadata.metadata_version,
    reference_set_version: null,
    input_digest: digest,
    context: {
      platform: "windows_steam",
      mode: "profile",
      build: null,
      dlc: null,
      source: "user_declared",
      limitations: ["profile.dat 是资料文件：只做容器与原始观察（模拟数据）"],
    },
    capabilities: {
      container: { status: "supported", reason_codes: [] },
      sections: { status: "unsupported", reason_codes: ["UNSUPPORTED_CONTEXT"] },
      inventory: { status: "unsupported", reason_codes: ["UNSUPPORTED_CONTEXT"] },
      counters: { status: "unsupported", reason_codes: ["UNSUPPORTED_CONTEXT"] },
      reference_difference: { status: "unsupported", reason_codes: ["UNSUPPORTED_CONTEXT"] },
      location: { status: "unsupported", reason_codes: ["UNSUPPORTED_CONTEXT"] },
    },
    coverage: {
      supported_category_ids: [],
      reference_category_ids: [],
      supported_region_counter_keys: [],
      unavailable_reason_codes: ["UNSUPPORTED_CONTEXT"],
      evidence_refs: [],
    },
    observations: {
      container: {
        status: "supported",
        format: "zlib",
        valid: true,
        eof: true,
        compressed_bytes: seed.compressedBytes,
        expanded_bytes: 25_600,
        trailing_bytes: 0,
        error: null,
      },
      sections: {
        status: "unsupported",
        valid: false,
        byte_length: 0,
        byte_length_hex: "0x0",
        start_offset: 0,
        start_offset_hex: "0x0",
        section_count: 0,
        error: null,
      },
      overall_completion: {
        value: null,
        unit: "percent",
        support_status: "unsupported",
        reason: "资料文件不做进度推定（模拟数据）",
        evidence_level: "observed",
        evidence_refs: [],
        limitations: [],
      },
      inventory: {
        status: "unsupported",
        inventory_complete: false,
        completeness_scope: "资料文件不恢复战役物品表（模拟数据）",
        records: [],
        record_count: 0,
        unique_id_count: 0,
        duplicate_record_count: 0,
      },
      region_counters: [],
    },
    assessments: [],
    locations: [],
    evidence: [],
    diagnostics: [
      {
        code: "MODE_PROFILE",
        severity: "info",
        scope: "input",
        message: "profile.dat 明确标为资料文件，不能通过用途选项被当作战役槽（模拟数据）",
        evidence_refs: [],
      },
    ],
  };
}

/** 非零计数组（模拟 save204.dat）：直接使用契约夹具。 */
export function buildNonzeroReport(seed: ReportSeed): Report {
  return { ...nonzeroFixture, input_digest: inputDigest(seed) };
}

/** 表不完整/计数待核对（模拟 save206.dat）：直接使用契约夹具。 */
export function buildDegradedReport(seed: ReportSeed): Report {
  return { ...degradedFixture, input_digest: inputDigest(seed) };
}

/** 首次分析失败的错误（模拟挑战古墓/损坏文件）。 */
export function mockInputError(code: string, message: string, retryable: boolean): InputError {
  return { code, message, retryable };
}
