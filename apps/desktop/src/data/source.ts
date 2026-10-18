// 数据源抽象（SX-015 起）：界面只依赖本接口，不直接调用 Tauri 命令。
//
// - `MockDataSource`（模拟数据源：契约夹具 + 演示输入，界面标注“模拟数据”）；
// - `TauriDataSource`（SX-016 真实接线）：把同样的请求映射到 SX-014 的最小权限
//   命令（payload 字段名与命令参数一致：camelCase 键 → Rust snake_case 参数；
//   命令返回保持 snake_case）；
// - 请求隔离由上层 store 负责：只有当前 request_id 的结果可以提交到当前槽位；
// - 导出与攻略链接都绑定 `snapshot_id` + `input_ref`，与当前报告同一快照。
import type { DesktopResult } from "../contract/desktop-result";

/**
 * 输入级错误（契约字段；生成类型未导出命名接口，此处按 schema 结构显式声明）。
 */
export interface InputError {
  code: string;
  message: string;
  retryable: boolean;
}

/** 命令级错误（SX-014 的 `CommandError`：稳定错误码 + 可展示说明）。 */
export interface CommandErrorDto {
  code: string;
  message: string;
}

/** Documents 候选目录（同 SX-014 `discover_saves` 返回形状）。 */
export interface DocumentsCandidateDto {
  path: string;
  source: string;
  source_label: string;
  status:
    | "game_found"
    | "no_game_dir"
    | "link_not_followed"
    | "missing"
    | "not_directory"
    | "unreadable"
    | "unsupported_location";
  detail: string | null;
}

export interface DiscoverOutcomeDto {
  candidates: DocumentsCandidateDto[];
  game_roots: string[];
}

/** 候选存档文件（同 SX-014 `list_candidates` 返回形状）。 */
export interface CandidateFileDto {
  name: string;
  path: string;
  size_bytes: number | null;
  modified_millis: number | null;
  matches_observed_names: boolean;
  status: "ready" | "too_large" | "unreadable";
  detail: string | null;
  /** 界面提示（后端可选提供，例如“内容与某文件相同”）；当前由模拟数据给出。 */
  note: string | null;
}

export interface SkippedPathDto {
  path: string;
  reason: "link_or_reparse_point" | "unreadable";
  detail: string | null;
}

export interface CandidatesOutcomeDto {
  root: string;
  files: CandidateFileDto[];
  skipped: SkippedPathDto[];
  depth_limited_dirs: string[];
}

export type Purpose = "campaign" | "unknown";

/** 选择器的用途（真实命令只有 file/directory 两种；用途决定模拟返回值与界面落点）。 */
export type PickKind = "save-file" | "save-directory" | "export-directory";

export interface AnalyzeRequest {
  requestId: string;
  path: string;
  purpose: Purpose;
}

export type ExportFormat = "json" | "csv" | "both";

/** 导出请求（同 SX-014 `export_report` 参数）。 */
export interface ExportRequest {
  snapshotId: string;
  inputRef: string;
  /** 必须是尚不存在的新目录（导出不覆盖任何内容）。 */
  outputDir: string;
  format: ExportFormat;
  /** 分享脱敏（默认开启；关闭需要用户显式取消勾选）。 */
  share: boolean;
}

export interface ExportArtifactDto {
  name: string;
  path: string;
}

/** 导出结果（同 SX-014 `ExportOutcome`）。 */
export interface ExportOutcomeDto {
  snapshot_id: string;
  input_ref: string;
  freshness: "current" | "refreshing" | "stale" | "none";
  last_refresh_error_code: string | null;
  share: boolean;
  artifacts: ExportArtifactDto[];
}

/** 攻略链接请求（同 SX-014 `resolve_guide_link`/`open_guide_link` 参数）。 */
export interface GuideLinkRequest {
  snapshotId: string;
  locationId: string;
  sourceRef: string;
}

/** 已校验的攻略链接（同 SX-014 `GuideLink`）。 */
export interface GuideLinkDto {
  snapshot_id: string;
  input_ref: string;
  location_id: string;
  source_ref: string;
  url: string;
  freshness: "current" | "refreshing" | "stale" | "none";
  action_kind: "whole_group_check" | "general_reference" | "unavailable" | null;
  action_reason_codes: string[];
}

export interface DataSource {
  /** `mock` 用于界面标注模拟数据；`tauri` 为真实接线。 */
  readonly kind: "mock" | "tauri";
  /** 发现候选目录（受限位置，只读）。 */
  discover(): Promise<DiscoverOutcomeDto>;
  /** 列出候选文件（有界枚举，不读取内容）。 */
  listCandidates(root: string): Promise<CandidatesOutcomeDto>;
  /** 用户主动选择文件/文件夹（取消返回 null）；kind 表示选择用途。 */
  pick(kind: PickKind): Promise<string | null>;
  /** 分析单个存档（返回 SX-003 桌面信封；取消返回无结果信封）。 */
  analyze(request: AnalyzeRequest): Promise<DesktopResult>;
  /** 取消在飞请求（幂等；被取消的请求不产生成功报告）。 */
  cancel(requestId: string): void | Promise<void>;
  /** 把当前快照导出为新目录中的 JSON/CSV（不覆盖存档或既有报告）。 */
  exportReport(request: ExportRequest): Promise<ExportOutcomeDto>;
  /** 仅解析（校验）攻略来源链接，不打开系统浏览器。 */
  resolveGuideLink(request: GuideLinkRequest): Promise<GuideLinkDto>;
  /** 解析并交给系统外部浏览器（WebView 不加载网页）。 */
  openGuideLink(request: GuideLinkRequest): Promise<GuideLinkDto>;
  /**
   * 打开界面合成的外部链接（搜索地址等）：不绑定快照，只做 http/https 白名单校验
   * 后交给系统外部浏览器。拒绝时以 `LINK_URL_UNSUPPORTED` / `LINK_OPEN_FAILED` 拒绝。
   */
  openExternalUrl(url: string): Promise<void>;
}
