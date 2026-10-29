/* 由 apps/desktop/scripts/contract/generate-types.mjs 从 schemas/export.schema.json 生成，请勿手改。契约说明见 docs/contracts.md。 */

/**
 * SOTTR Explorer 导出信封。导出文件必须自带快照身份、freshness 与最近刷新错误码；不能只在界面贴“过期”而文件无标识。普通核心 JSON 与导出信封各自有 schema。
 */
export interface ExportDocument {
  schema_version: "1.0.0";
  document_kind: "report_export";
  input_ref: string;
  snapshot_id: string;
  snapshot_time: string;
  freshness: "current" | "refreshing" | "stale";
  last_refresh_error_code: string | null;
  input_digest: InputDigest;
  report: {
    schema_version: "1.0.0";
    tool_version: string;
    parser_profile_id: string;
    /**
     * 未知为 null；原因写入 diagnostics 的 METADATA_NOT_LOADED 等条目，不虚构版本号
     */
    metadata_version: string | null;
    /**
     * 未知为 null；原因写入 diagnostics，不虚构版本号
     */
    reference_set_version: string | null;
    input_digest: InputDigest;
    /**
     * 用户声明是上下文证据，不是格式识别证明
     */
    context: {
      /**
       * 如 windows_steam；未知为 null
       */
      platform: string | null;
      mode: "campaign" | "profile" | "unknown" | "unsupported";
      /**
       * 未知为 null，不虚构构建号
       */
      build: string | null;
      dlc: string[] | null;
      source: "user_declared" | "fixture_provenance" | "unknown";
      limitations: string[];
    };
    capabilities: {
      container: Sections;
      sections: Sections;
      inventory: Sections;
      counters: Sections;
      reference_difference: Sections;
      location: Sections;
    };
    /**
     * 43 项稀疏计数不通过数组填充变成完整矩阵；汇总计数必须携带 scope 与支持状态
     */
    coverage: {
      supported_category_ids: string[];
      reference_category_ids: string[];
      supported_region_counter_keys: string[];
      unavailable_reason_codes: string[];
      evidence_refs: string[];
    };
    observations: {
      container: {
        /**
         * 字段在当前报告上下文中的可用性；unsupported 时 value 必为 null 且 reason 必填
         */
        status: "supported" | "partial" | "unsupported";
        format: string;
        valid: boolean;
        eof: boolean;
        compressed_bytes: number;
        expanded_bytes: number | null;
        trailing_bytes: number;
        error: string | null;
      };
      sections: {
        /**
         * 字段在当前报告上下文中的可用性；unsupported 时 value 必为 null 且 reason 必填
         */
        status: "supported" | "partial" | "unsupported";
        valid: boolean;
        byte_length: number;
        byte_length_hex: string;
        start_offset: number;
        start_offset_hex: string;
        section_count: number;
        error: string | null;
      };
      /**
       * 未知值 value 为 null 且 reason 必填；合法零保留 0
       */
      overall_completion: {
        value: number | null;
        unit: string;
        /**
         * 字段在当前报告上下文中的可用性；unsupported 时 value 必为 null 且 reason 必填
         */
        support_status: "supported" | "partial" | "unsupported";
        reason: string | null;
        /**
         * 证据来源等级；禁止无依据概率分数
         */
        evidence_level: "observed" | "community_correlated" | "game_validated";
        evidence_refs: string[];
        limitations: string[];
      };
      inventory: {
        /**
         * 字段在当前报告上下文中的可用性；unsupported 时 value 必为 null 且 reason 必填
         */
        status: "supported" | "partial" | "unsupported";
        /**
         * 仅指当前 parser profile 下表恢复完整，不代表全游戏全集完备
         */
        inventory_complete: boolean;
        completeness_scope: string;
        /**
         * Items: 重复 ID 保留全部原始记录；未知非零 ID 不丢弃；伴随字含义未确立，不赋状态语义
         */
        records: {
          table: "prefix" | "type28_overflow";
          index: number;
          /**
           * u32 内部 ID，固定 0x 加 8 位大写十六进制
           */
          internal_id: string;
          raw_companion_u32: number;
          raw_companion_hex: string;
          raw_low_two_bits: number;
          id_offset: number;
          id_offset_hex: string;
        }[];
        record_count: number;
        unique_id_count: number;
        duplicate_record_count: number;
      };
      /**
       * Items: 计数超过来源目标时保留 raw_value，value 与剩余置 null，不裁剪到目标值
       */
      region_counters: {
        counter_key: string;
        region_id: string | null;
        category_id: string;
        value: number | null;
        unit: string;
        /**
         * 字段在当前报告上下文中的可用性；unsupported 时 value 必为 null 且 reason 必填
         */
        support_status: "supported" | "partial" | "unsupported";
        reason: string | null;
        /**
         * 证据来源等级；禁止无依据概率分数
         */
        evidence_level: "observed" | "community_correlated" | "game_validated";
        evidence_refs: string[];
        limitations: string[];
        source_target: number | null;
        within_source_target: boolean;
        raw_value: number;
        raw_value_hex: string;
      }[];
    };
    /**
     * Items: 身份不等于状态；原始 present 默认仅表示已观察
     */
    assessments: {
      /**
       * u32 内部 ID，固定 0x 加 8 位大写十六进制
       */
      internal_id: string;
      observation: "present" | "reference_present_current_absent" | "unknown";
      /**
       * 首版无新游戏内证据时不产生 verified_*；likely_* 仅在对应规则全部满足时出现
       */
      assessment: "likely_collected" | "likely_missing" | "unknown" | "verified_collected" | "verified_uncollected";
      rule_id: string | null;
      evidence_refs: string[];
      limitations: string[];
    }[];
    /**
     * Items: 成员关系（member_ids）与精确条目映射（guide_item）分开；无已验证映射时 coordinates 为 null 且不伪造精度
     */
    locations: {
      location_id: string;
      precision: "region" | "guide_group" | "guide_item" | "verified_coordinates";
      region_id: string | null;
      guide_group_id: string | null;
      guide_item: string | null;
      /**
       * Items: u32 内部 ID，固定 0x 加 8 位大写十六进制
       */
      member_ids: string[];
      target_count: number | null;
      mapping_kind: "region_clue" | "guide_group_membership" | "exact_guide_item" | "verified_coordinates" | "none";
      source_refs: string[];
      reference_set_version: string | null;
      applicable_mode: "campaign" | "profile" | "unknown" | "unsupported" | null;
      applicable_platform: string | null;
      coordinates: {
        x: number;
        y: number;
        z: number;
        coordinate_space: string;
        unit: string;
        level_id: string;
      } | null;
      map_transform: {
        basemap_version: string;
        /**
         * JSON 字符串，内容是 schemas/basemap.schema.json 的 Transform 对象：kind/matrix/coordinate_y_axis/image_origin。定义见 docs/contracts.md「BasemapDataset」一节
         */
        transform_parameters: string;
        control_points_source: string;
        error_description: string;
      } | null;
      group_actionability: {
        kind: "whole_group_check" | "general_reference" | "unavailable";
        reason_codes: string[];
        evidence_refs: string[];
        evaluated_input_digest: InputDigest;
      } | null;
      limitations: string[];
      reason: string | null;
    }[];
    evidence: {
      evidence_id: string;
      method: string;
      source_refs: string[];
      sample_scope: string;
      limitations: string[];
      offset: number | null;
      offset_hex: string | null;
      record_index: number | null;
      reference_set_id: string | null;
      cross_checks: string[];
      /**
       * 来源版本表：来源标识到固定提交/文档版本；无则为空对象
       */
      source_versions: {
        [k: string]: string;
      };
    }[];
    /**
     * Items: UI 不依赖解析自然语言决定是否允许推定，只看 code/severity/scope
     */
    diagnostics: {
      code: string;
      severity: "info" | "warning" | "error";
      scope:
        | "input"
        | "container"
        | "sections"
        | "inventory"
        | "counters"
        | "reference_difference"
        | "location"
        | "metadata";
      message: string;
      evidence_refs: string[];
    }[];
  };
}
/**
 * 个人绝对路径不属于契约字段
 */
export interface InputDigest {
  compressed_sha256: string;
  expanded_sha256: string | null;
  compressed_bytes: number;
  expanded_bytes: number | null;
}
export interface Sections {
  /**
   * 字段在当前报告上下文中的可用性；unsupported 时 value 必为 null 且 reason 必填
   */
  status: "supported" | "partial" | "unsupported";
  reason_codes: string[];
}
