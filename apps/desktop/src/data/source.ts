// 数据源抽象（SX-015）：界面只依赖本接口，不直接调用 Tauri 命令。
//
// - 本次（SX-015）提供 `MockDataSource`（契约夹具 + 演示数据，界面标注“模拟数据”）；
// - SX-016 将提供 `TauriDataSource`，把同样的请求映射到 SX-014 的最小权限命令
//   （payload 字段名与命令返回保持 snake_case 一致，界面无需改动）；
// - 请求隔离由上层 store 负责：只有当前 request_id 的结果可以提交到当前槽位。
import type { DesktopResult } from "../contract/desktop-result";

/**
 * 输入级错误（契约字段；生成类型未导出命名接口，此处按 schema 结构显式声明）。
 */
export interface InputError {
  code: string;
  message: string;
  retryable: boolean;
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
  /** 界面提示（后端可选提供，例如“内容与某文件相同”）；真实接线前由模拟数据给出。 */
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

export interface AnalyzeRequest {
  requestId: string;
  path: string;
  purpose: Purpose;
}

export interface DataSource {
  /** `mock` 用于界面标注模拟数据；`tauri` 为 SX-016 的真实接线。 */
  readonly kind: "mock" | "tauri";
  /** 发现候选目录（受限位置，只读）。 */
  discover(): Promise<DiscoverOutcomeDto>;
  /** 列出候选文件（有界枚举，不读取内容）。 */
  listCandidates(root: string): Promise<CandidatesOutcomeDto>;
  /** 用户主动选择文件/文件夹（取消返回 null）。 */
  pick(kind: "file" | "directory"): Promise<string | null>;
  /** 分析单个存档（返回 SX-003 桌面信封；取消返回无结果信封）。 */
  analyze(request: AnalyzeRequest): Promise<DesktopResult>;
  /** 取消在飞请求（幂等；被取消的请求不产生成功报告）。 */
  cancel(requestId: string): void;
}
